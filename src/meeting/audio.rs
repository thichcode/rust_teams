//! Audio capture module using cpal
//! Captures microphone and system audio, mixes them into a single buffer

#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SampleRate, Stream, StreamConfig};

use super::config::AudioConfig;

/// Audio capture manager
pub struct AudioCapture {
    mic_stream: Option<Stream>,
    system_stream: Option<Stream>,
    buffer: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    config: AudioConfig,
}

impl AudioCapture {
    /// Create new audio capture
    pub fn new(config: AudioConfig) -> Result<Self> {
        Ok(Self {
            mic_stream: None,
            system_stream: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(AtomicBool::new(false)),
            config,
        })
    }

    /// Start recording audio
    pub fn start_recording(&mut self) -> Result<()> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Ok(());
        }

        let host = cpal::default_host();

        // Clear buffer
        {
            let mut buffer = self.buffer.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            buffer.clear();
        }

        // Start microphone capture
        if self.config.record_microphone {
            if let Some(device) = host.default_input_device() {
                log::info!("Using microphone: {}", device.name().unwrap_or_default());
                self.mic_stream = Some(self.start_input_stream(&device)?);
            } else {
                log::warn!("No microphone found");
            }
        }

        // Start system audio capture (WASAPI loopback on Windows)
        if self.config.record_system_audio {
            if let Some(device) = self.get_loopback_device(&host) {
                log::info!("Using system audio loopback");
                self.system_stream = Some(self.start_input_stream(&device)?);
            } else {
                log::warn!("No system audio loopback available");
            }
        }

        self.is_recording.store(true, Ordering::Relaxed);
        log::info!("Audio recording started");

        Ok(())
    }

    /// Stop recording and return the captured audio
    pub fn stop_recording(&mut self) -> Result<Vec<f32>> {
        self.is_recording.store(false, Ordering::Relaxed);

        // Drop streams to stop recording
        self.mic_stream.take();
        self.system_stream.take();

        // Get buffer
        let buffer = {
            let mut buf = self.buffer.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            std::mem::take(&mut *buf)
        };

        log::info!("Audio recording stopped, {} samples", buffer.len());
        Ok(buffer)
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    /// Get current buffer length
    pub fn buffer_len(&self) -> usize {
        self.buffer.lock().map(|b| b.len()).unwrap_or(0)
    }

    /// Get loopback device for system audio capture (Windows)
    fn get_loopback_device(&self, host: &cpal::Host) -> Option<Device> {
        // On Windows, look for loopback device
        #[cfg(target_os = "windows")]
        {
            // Try to find WASAPI loopback device
            for device in host.devices().ok()? {
                if let Ok(name) = device.name() {
                    if name.contains("loopback") || name.contains("Stereo Mix") {
                        return Some(device);
                    }
                }
            }
        }

        // Fallback: use default input device
        host.default_input_device()
    }

    /// Start an input stream on a device
    fn start_input_stream(&self, device: &Device) -> Result<Stream> {
        let _supported_config = device
            .supported_input_configs()
            .map_err(|e| anyhow::anyhow!("Failed to get configs: {}", e))?
            .find(|c| c.sample_format() == SampleFormat::F32)
            .or_else(|| {
                device
                    .supported_input_configs()
                    .ok()?
                    .next()
            })
            .ok_or_else(|| anyhow::anyhow!("No supported audio config"))?;

        let config = StreamConfig {
            channels: self.config.channels,
            sample_rate: SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer = self.buffer.clone();
        let is_recording = self.is_recording.clone();

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if is_recording.load(Ordering::Relaxed) {
                        if let Ok(mut buf) = buffer.lock() {
                            buf.extend_from_slice(data);
                        }
                    }
                },
                |err| {
                    log::error!("Audio stream error: {}", err);
                },
            )
            .map_err(|e| anyhow::anyhow!("Failed to build stream: {}", e))?;

        stream
            .play()
            .map_err(|e| anyhow::anyhow!("Failed to start stream: {}", e))?;

        Ok(stream)
    }

    /// Convert audio samples to WAV bytes
    pub fn to_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<u8>> {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut buffer, spec)
                .map_err(|e| anyhow::anyhow!("Failed to create WAV writer: {}", e))?;

            for &sample in samples {
                let scaled = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                writer
                    .write_sample(scaled)
                    .map_err(|e| anyhow::anyhow!("Failed to write sample: {}", e))?;
            }

            writer
                .finalize()
                .map_err(|e| anyhow::anyhow!("Failed to finalize WAV: {}", e))?;
        }

        Ok(buffer.into_inner())
    }
}
