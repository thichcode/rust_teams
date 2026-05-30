//! Meeting notes module
//! Auto-generate meeting notes from Teams audio

pub mod audio;
pub mod config;
pub mod detect;
pub mod llm;
pub mod notes;
pub mod stt;

pub use config::MeetingNotesConfig;
pub use notes::{MeetingNotes, MeetingNotesGenerator};
pub use detect::{MeetingDetector, MeetingState};
