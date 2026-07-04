/// Round-robin speaker labeler (NOT real voice diarization).
///
/// Alternates between "Speaker 1" and "Speaker 2" on each utterance.
/// A proper diarization implementation would use embedding similarity
/// (e.g., pyannote-audio) — tracked in ROADMAP.
pub struct SpeakerLabeler {
    current_speaker: usize,
    total_speakers: usize,
    utterance_count: usize,
}

impl SpeakerLabeler {
    pub fn new() -> Self {
        Self {
            current_speaker: 0,
            total_speakers: 2,
            utterance_count: 0,
        }
    }

    pub fn next_utterance(&mut self) -> String {
        self.utterance_count += 1;
        self.current_speaker = (self.utterance_count - 1) % self.total_speakers;
        self.label_for(self.current_speaker)
    }

    fn label_for(&self, idx: usize) -> String {
        format!("Speaker {}", idx + 1)
    }

    #[allow(dead_code)]
    pub fn current_label(&self) -> String {
        self.label_for(self.current_speaker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alternates_between_two_speakers() {
        let mut s = SpeakerLabeler::new();
        assert_eq!(s.next_utterance(), "Speaker 1");
        assert_eq!(s.next_utterance(), "Speaker 2");
        assert_eq!(s.next_utterance(), "Speaker 1");
        assert_eq!(s.next_utterance(), "Speaker 2");
    }

    #[test]
    fn test_new_starts_at_speaker_1() {
        let s = SpeakerLabeler::new();
        assert_eq!(s.current_label(), "Speaker 1");
    }
}
