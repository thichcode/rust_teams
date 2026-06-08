pub struct Vad {
    threshold: f32,
    noise_floor: f32,
    adapted_frames: u32,
}

impl Vad {
    pub fn new() -> Self {
        Self {
            threshold: 3.0,
            noise_floor: 0.003,
            adapted_frames: 0,
        }
    }

    pub fn is_voice(&mut self, samples: &[f32]) -> bool {
        let rms = self.compute_rms(samples);
        self.update_noise_floor(rms);
        rms > self.noise_floor * self.threshold
    }

    fn compute_rms(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum: f32 = samples.iter().map(|s| s * s).sum();
        (sum / samples.len() as f32).sqrt()
    }

    fn update_noise_floor(&mut self, rms: f32) {
        if self.adapted_frames < 200 {
            self.noise_floor = self.noise_floor * 0.7 + rms * 0.3;
            self.adapted_frames += 1;
        } else if rms < self.noise_floor {
            self.noise_floor = self.noise_floor * 0.99 + rms * 0.01;
        }
    }
}
