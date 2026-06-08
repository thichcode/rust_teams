pub struct Diarizer {
    current_speaker: usize,
    total_speakers: usize,
    utterance_count: usize,
}

impl Diarizer {
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

    pub fn label_for(&self, idx: usize) -> String {
        format!("Speaker {}", idx + 1)
    }

    #[allow(dead_code)]
    pub fn current_label(&self) -> String {
        self.label_for(self.current_speaker)
    }
}
