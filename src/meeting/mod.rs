//! Meeting notes module
//! Auto-generate meeting notes from Teams audio
//!
//! Also hosts the realtime translate pipeline used to:
//!   - capture audio chunks from a live call
//!   - transcribe via STT
//!   - translate into a target language
//!   - suggest short replies the user can say next

#![allow(unused_imports)]

pub mod audio;
pub mod config;
pub mod detect;
pub mod llm;
pub mod notes;
pub mod realtime;
pub mod realtime_config;
pub mod stt;
pub mod suggester;
pub mod translator;
pub mod whisper_download;

pub use config::MeetingNotesConfig;
pub use notes::{MeetingNotes, MeetingNotesGenerator};
pub use detect::{MeetingDetector, MeetingState};
pub use realtime::{RealtimePayload, RealtimeTranslatePipeline};
pub use realtime_config::RealtimeTranslateConfig;
