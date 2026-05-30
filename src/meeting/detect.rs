//! Meeting detection module
//! Detects when a Teams meeting starts and ends

#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Meeting state
#[derive(Debug, Clone, PartialEq)]
pub enum MeetingState {
    /// No meeting active
    Idle,
    /// Meeting detected, waiting to confirm
    Detecting,
    /// Meeting is active
    Active,
    /// Meeting just ended, cooling down
    CoolingDown,
}

/// Meeting detector
pub struct MeetingDetector {
    state: Arc<Mutex<MeetingState>>,
    last_activity: Arc<Mutex<Instant>>,
    is_meeting: Arc<AtomicBool>,
    silence_threshold: Duration,
    activity_timeout: Duration,
}

impl MeetingDetector {
    /// Create new meeting detector
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MeetingState::Idle)),
            last_activity: Arc::new(Mutex::new(Instant::now())),
            is_meeting: Arc::new(AtomicBool::new(false)),
            silence_threshold: Duration::from_secs(3),
            activity_timeout: Duration::from_secs(30),
        }
    }

    /// Check if window title indicates a meeting
    pub fn check_title(&self, title: &str) -> bool {
        let meeting_indicators = [
            "Meeting",
            "Call",
            "Teams",
            "In a call",
            "In a meeting",
            "Presenting",
            "Screen sharing",
        ];

        meeting_indicators.iter().any(|&indicator| {
            title.to_lowercase().contains(&indicator.to_lowercase())
        })
    }

    /// Update meeting state based on activity
    pub fn update_activity(&self, has_audio: bool) -> bool {
        let mut state = self.state.lock().unwrap();
        let mut last_activity = self.last_activity.lock().unwrap();

        let was_meeting = self.is_meeting.load(Ordering::Relaxed);

        match *state {
            MeetingState::Idle => {
                if has_audio {
                    *state = MeetingState::Detecting;
                    *last_activity = Instant::now();
                }
            }
            MeetingState::Detecting => {
                if has_audio {
                    // Confirmed meeting
                    *state = MeetingState::Active;
                    self.is_meeting.store(true, Ordering::Relaxed);
                    log::info!("Meeting detected and confirmed");
                } else if last_activity.elapsed() > self.silence_threshold {
                    // Too much silence, back to idle
                    *state = MeetingState::Idle;
                }
            }
            MeetingState::Active => {
                if has_audio {
                    *last_activity = Instant::now();
                } else if last_activity.elapsed() > self.activity_timeout {
                    // No activity for a while, meeting might have ended
                    *state = MeetingState::CoolingDown;
                    *last_activity = Instant::now();
                    log::info!("Meeting activity stopped, cooling down...");
                }
            }
            MeetingState::CoolingDown => {
                if has_audio {
                    // Audio resumed, meeting still active
                    *state = MeetingState::Active;
                    *last_activity = Instant::now();
                } else if last_activity.elapsed() > Duration::from_secs(10) {
                    // No activity for 10s, confirm meeting ended
                    *state = MeetingState::Idle;
                    self.is_meeting.store(false, Ordering::Relaxed);
                    log::info!("Meeting ended");
                    return true; // Meeting ended
                }
            }
        }

        let is_meeting = self.is_meeting.load(Ordering::Relaxed);
        is_meeting != was_meeting
    }

    /// Get current state
    pub fn state(&self) -> MeetingState {
        self.state.lock().unwrap().clone()
    }

    /// Check if meeting is active
    pub fn is_meeting(&self) -> bool {
        self.is_meeting.load(Ordering::Relaxed)
    }

    /// Check if audio has activity (not silence)
    pub fn detect_audio_activity(samples: &[f32], threshold: f32) -> bool {
        if samples.is_empty() {
            return false;
        }

        // Calculate RMS (Root Mean Square)
        let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
        let rms = (sum_squares / samples.len() as f32).sqrt();

        rms > threshold
    }
}

impl Default for MeetingDetector {
    fn default() -> Self {
        Self::new()
    }
}
