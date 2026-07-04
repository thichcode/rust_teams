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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_is_not_voice() {
        let mut vad = Vad::new();
        let silence = vec![0.0001_f32; 480];
        assert!(!vad.is_voice(&silence));
    }

    #[test]
    fn test_loud_is_voice() {
        let mut vad = Vad::new();
        let loud = vec![0.5_f32; 480];
        assert!(vad.is_voice(&loud));
    }

    #[test]
    fn test_adaptation() {
        let mut vad = Vad::new();
        // After 200 frames of loud noise, noise floor rises
        for _ in 0..250 {
            let frame = vec![0.1_f32; 480];
            vad.is_voice(&frame);
        }
        // Now 0.1 should be below threshold
        let frame = vec![0.05_f32; 480];
        assert!(!vad.is_voice(&frame));
    }

    #[test]
    fn test_compute_rms_silence() {
        let vad = Vad::new();
        let rms = vad.compute_rms(&[0.0_f32; 100]);
        assert!(rms < 0.001);
    }

    #[test]
    fn test_compute_rms_loud() {
        let vad = Vad::new();
        let rms = vad.compute_rms(&[0.5_f32; 100]);
        assert!((rms - 0.5).abs() < 0.01);
    }
}
