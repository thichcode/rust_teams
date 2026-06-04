//! WASAPI system audio loopback capture (Windows only).
//!
//! Captures the audio being played to the default render device
//! (speakers / headphones) by opening a loopback client on it.
//! Decodes float32 mono samples at 16 kHz and pushes them into a
//! shared buffer used by the realtime translate pipeline.
//!
//! See: <https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording>

#![cfg(windows)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use wasapi::{
    get_default_device, AudioCaptureClient, Direction, Handle, SampleType, StreamMode, WaveFormat,
};

const LOOPBACK_SAMPLE_RATE: u32 = 16_000;
const LOOPBACK_CHANNELS: u16 = 1;

/// Handle to a running loopback capture. Drop or call `stop()` to terminate.
pub struct LoopbackHandle {
    stop_flag: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    stop_tx: Option<mpsc::Sender<()>>,
}

impl LoopbackHandle {
    /// Stop the capture thread and wait for it to finish.
    pub fn stop(mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for LoopbackHandle {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Start capturing the default render device and push float32 mono
/// samples (at 16 kHz) into `buffer`.
///
/// `buffer` should be the same `Arc<Mutex<Vec<f32>>>` shared with the
/// downstream consumer (e.g. `AudioCapture`).
pub fn start_loopback(buffer: Arc<Mutex<Vec<f32>>>) -> Result<LoopbackHandle> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    let stop_flag_thread = stop_flag.clone();
    let join = thread::Builder::new()
        .name("wasapi-loopback".to_string())
        .spawn(move || {
            if let Err(e) = run_loopback(buffer, stop_flag_thread, stop_rx) {
                log::error!("WASAPI loopback stopped with error: {e:#}");
            }
        })
        .context("failed to spawn WASAPI loopback thread")?;

    Ok(LoopbackHandle {
        stop_flag,
        join: Some(join),
        stop_tx: Some(stop_tx),
    })
}

fn run_loopback(
    buffer: Arc<Mutex<Vec<f32>>>,
    stop_flag: Arc<AtomicBool>,
    stop_rx: mpsc::Receiver<()>,
) -> Result<()> {
    // WASAPI requires COM. Use multi-threaded apartment on this worker
    // thread so we don't conflict with any STA on the main thread.
    let hr = wasapi::initialize_mta();
    if hr.is_err() {
        anyhow::bail!("CoInitializeEx(MTA) failed: {:?}", hr);
    }

    // Get default render (speaker/headphone) device.
    let device = get_default_device(&Direction::Render)
        .map_err(|e| anyhow::anyhow!("failed to get default render device: {e:?}"))?;
    let device_name = device
        .get_description()
        .unwrap_or_else(|_| "<unknown>".to_string());
    log::info!("WASAPI loopback target device: {device_name}");

    let mut audio_client = device
        .get_iaudioclient()
        .map_err(|e| anyhow::anyhow!("get_iaudioclient failed: {e:?}"))?;

    // Request 16 kHz mono float32 — small payload for the STT pipeline.
    let desired = WaveFormat::new(
        32,
        32,
        &SampleType::Float,
        LOOPBACK_SAMPLE_RATE as usize,
        LOOPBACK_CHANNELS as usize,
        None,
    );

    // Pair (Render, Capture) in shared mode => AUDCLNT_STREAMFLAGS_LOOPBACK
    // is set automatically by wasapi.
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: 0,
    };
    audio_client
        .initialize_client(&desired, &Direction::Capture, &mode)
        .map_err(|e| anyhow::anyhow!("initialize_client (loopback) failed: {e:?}"))?;

    let h_event: Handle = audio_client
        .set_get_eventhandle()
        .map_err(|e| anyhow::anyhow!("set_get_eventhandle failed: {e:?}"))?;
    let buffer_frame_count = audio_client
        .get_buffer_size()
        .map_err(|e| anyhow::anyhow!("get_buffer_size failed: {e:?}"))?;
    log::info!(
        "WASAPI loopback initialized, buffer_size={buffer_frame_count} frames @ {} Hz",
        LOOPBACK_SAMPLE_RATE
    );

    let capture_client: AudioCaptureClient = audio_client
        .get_audiocaptureclient()
        .map_err(|e| anyhow::anyhow!("get_audiocaptureclient failed: {e:?}"))?;

    audio_client
        .start_stream()
        .map_err(|e| anyhow::anyhow!("start_stream failed: {e:?}"))?;

    let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(
        4 * 1024 + (LOOPBACK_SAMPLE_RATE as usize / 5) * 4,
    );

    while !stop_flag.load(Ordering::Relaxed) {
        // Wake on event (timeout 100ms so we can check stop_flag).
        if h_event.wait_for_event(100).is_err() {
            // Timed out — just loop and check stop flag.
        }

        // Pull all available samples into our local deque.
        loop {
            match capture_client.read_from_device_to_deque(&mut sample_queue) {
                Ok(info) => {
                    if info.flags.silent {
                        // Device is producing silence for this buffer.
                        log::trace!("WASAPI loopback buffer marked SILENT");
                    }
                    if sample_queue.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    log::error!("read_from_device_to_deque error: {e:?}");
                    break;
                }
            }
        }

        // Flush queued bytes to f32 samples and push into shared buffer.
        if !sample_queue.is_empty() {
            let total_samples = sample_queue.len() / 4;
            if total_samples > 0 {
                let mut samples = Vec::with_capacity(total_samples);
                for _ in 0..total_samples {
                    let b0 = sample_queue.pop_front().unwrap_or(0);
                    let b1 = sample_queue.pop_front().unwrap_or(0);
                    let b2 = sample_queue.pop_front().unwrap_or(0);
                    let b3 = sample_queue.pop_front().unwrap_or(0);
                    let bits = u32::from_le_bytes([b0, b1, b2, b3]);
                    samples.push(f32::from_bits(bits));
                }
                if let Ok(mut buf) = buffer.lock() {
                    buf.extend_from_slice(&samples);
                }
            }
        }

        // Also wake up on stop signal.
        if stop_rx.try_recv().is_ok() {
            break;
        }
    }

    let _ = audio_client.stop_stream();
    log::info!("WASAPI loopback stopped");
    Ok(())
}
