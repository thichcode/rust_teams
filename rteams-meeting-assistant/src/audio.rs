use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleRate, Stream, StreamConfig};
use wasapi::{
    AudioCaptureClient, Direction, Handle, SampleType, StreamMode, WaveFormat, get_default_device,
};

pub struct LoopbackHandle {
    stop_flag: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl LoopbackHandle {
    pub fn stop(mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn start_wasapi_loopback(buffer: Arc<Mutex<Vec<f32>>>) -> Result<LoopbackHandle> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let sf = stop_flag.clone();
    let join = std::thread::Builder::new()
        .name("wasapi-loopback".to_string())
        .spawn(move || {
            let _ = wasapi::initialize_mta();
            let device = match get_default_device(&Direction::Render) {
                Ok(d) => d,
                Err(_) => return,
            };
            let mut client = match device.get_iaudioclient() {
                Ok(c) => c,
                Err(_) => return,
            };
            let fmt = WaveFormat::new(32, 32, &SampleType::Float, 16000, 1, None);
            let mode = StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: 0,
            };
            if client
                .initialize_client(&fmt, &Direction::Capture, &mode)
                .is_err()
            {
                return;
            }
            let h_event: Handle = match client.set_get_eventhandle() {
                Ok(h) => h,
                Err(_) => return,
            };
            let cap: AudioCaptureClient = match client.get_audiocaptureclient() {
                Ok(c) => c,
                Err(_) => return,
            };
            let _ = client.start_stream();

            let mut deque = std::collections::VecDeque::new();
            while !sf.load(Ordering::Relaxed) {
                let _ = h_event.wait_for_event(100);
                while cap.read_from_device_to_deque(&mut deque).is_ok() {
                    if deque.is_empty() {
                        break;
                    }
                }
                let n = deque.len() / 4;
                if n > 0 {
                    let mut samples = Vec::with_capacity(n);
                    for _ in 0..n {
                        let b0 = deque.pop_front();
                        let b1 = deque.pop_front();
                        let b2 = deque.pop_front();
                        let b3 = deque.pop_front();
                        match (b0, b1, b2, b3) {
                            (Some(b0), Some(b1), Some(b2), Some(b3)) => {
                                samples.push(f32::from_bits(u32::from_le_bytes([b0, b1, b2, b3])));
                            }
                            _ => {
                                log::error!(
                                    "WASAPI loopback: incomplete frame, dropping {n} samples"
                                );
                                deque.clear();
                                break;
                            }
                        }
                    }
                    if let Ok(mut buf) = buffer.lock() {
                        buf.extend_from_slice(&samples);
                    }
                }
            }
            let _ = client.stop_stream();
        })?;
    Ok(LoopbackHandle {
        stop_flag,
        join: Some(join),
    })
}

pub struct AudioCapture {
    mic_stream: Option<Stream>,
    loopback_handle: Option<LoopbackHandle>,
    buffer: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    audio_input_device: String,
    capture_system_audio: bool,
}

impl AudioCapture {
    pub fn new(audio_input_device: &str, capture_system_audio: bool) -> Self {
        Self {
            mic_stream: None,
            loopback_handle: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(AtomicBool::new(false)),
            audio_input_device: audio_input_device.to_string(),
            capture_system_audio,
        }
    }

    pub fn input_device_names() -> Vec<String> {
        let host = cpal::default_host();
        let mut names: Vec<String> = host
            .input_devices()
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|device| device.name().ok())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    pub fn start(&mut self) -> Result<()> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Ok(());
        }
        {
            let mut buf = self
                .buffer
                .lock()
                .map_err(|e| anyhow::anyhow!("lock: {}", e))?;
            buf.clear();
        }

        let host = cpal::default_host();
        let input_device = select_input_device(&host, &self.audio_input_device)?;
        if let Some(device) = input_device {
            let cfg = StreamConfig {
                channels: 1,
                sample_rate: SampleRate(16000),
                buffer_size: cpal::BufferSize::Default,
            };
            let buf = self.buffer.clone();
            let rec = self.is_recording.clone();
            let stream = device
                .build_input_stream(
                    &cfg,
                    move |data: &[f32], _: &_| {
                        if rec.load(Ordering::Relaxed)
                            && let Ok(mut b) = buf.lock()
                        {
                            b.extend_from_slice(data);
                        }
                    },
                    |e| log::error!("mic stream error: {e}"),
                )
                .map_err(|e| anyhow::anyhow!("mic stream: {e}"))?;
            stream
                .play()
                .map_err(|e| anyhow::anyhow!("mic play: {e}"))?;
            self.mic_stream = Some(stream);
        }

        if self.capture_system_audio {
            match start_wasapi_loopback(self.buffer.clone()) {
                Ok(h) => self.loopback_handle = Some(h),
                Err(e) => log::warn!("loopback failed: {e}"),
            }
        }

        if self.mic_stream.is_none() && self.loopback_handle.is_none() {
            anyhow::bail!("No audio capture source is available");
        }

        self.is_recording.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn stop(&mut self) -> Vec<f32> {
        self.is_recording.store(false, Ordering::Relaxed);
        self.mic_stream.take();
        if let Some(h) = self.loopback_handle.take() {
            h.stop();
        }
        self.buffer
            .lock()
            .map(|mut b| std::mem::take(&mut *b))
            .unwrap_or_default()
    }

    pub fn drain_buffer(&mut self) -> Vec<f32> {
        self.buffer
            .lock()
            .map(|mut b| std::mem::take(&mut *b))
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    pub fn to_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<u8>> {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut w = hound::WavWriter::new(&mut cursor, spec)?;
            for &s in samples {
                let scaled = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
                w.write_sample(scaled)?;
            }
            w.finalize()?;
        }
        Ok(cursor.into_inner())
    }
}

fn select_input_device(host: &cpal::Host, name: &str) -> Result<Option<Device>> {
    let wanted = name.trim();
    if wanted.is_empty() {
        return Ok(host.default_input_device());
    }

    let devices = host
        .input_devices()
        .map_err(|e| anyhow::anyhow!("input devices: {e}"))?;
    for device in devices {
        if device.name().ok().as_deref() == Some(wanted) {
            return Ok(Some(device));
        }
    }

    anyhow::bail!("Selected audio input not found: {wanted}")
}
