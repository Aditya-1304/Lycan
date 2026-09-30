//! Streaming conversion from the fixed core cadence to a host device rate.

use crate::PCM_RATE;

/// Linear interpolation with an integer phase carried across chunk boundaries.
/// Reset only on session discontinuities; ordinary frame boundaries retain it.
pub struct Resampler {
    output_rate: u32,
    previous: Option<f32>,
    phase: u64,
}

impl Resampler {
    /// Output adapters supply their actual device/AudioContext sample rate.
    pub fn new(output_rate: u32) -> Self {
        assert!(output_rate > 0);
        Self {
            output_rate,
            previous: None,
            phase: 0,
        }
    }

    /// Appends interleaving-independent mono samples into caller-owned storage.
    pub fn process(&mut self, input: &[f32], output: &mut Vec<f32>) {
        for &sample in input {
            if let Some(previous) = self.previous {
                while self.phase < u64::from(self.output_rate) {
                    let fraction = self.phase as f32 / self.output_rate as f32;
                    output.push(previous + (sample - previous) * fraction);
                    self.phase += u64::from(PCM_RATE);
                }
                self.phase -= u64::from(self.output_rate);
            }
            self.previous = Some(sample);
        }
    }

    /// Drops interpolation history after pause, focus loss, load or reset.
    pub fn reset(&mut self) {
        self.previous = None;
        self.phase = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Catches resetting phase at each app chunk and accumulated rate drift.
    // Core FIFO tests use the fixed 32768 Hz cadence and cannot detect either.
    #[test]
    fn uneven_chunks_match_continuous_resampling_at_device_rates() {
        let input: Vec<_> = (0..=PCM_RATE).map(|i| (i % 256) as f32 / 256.0).collect();
        for rate in [44_100, 48_000, 96_000] {
            let mut whole = Vec::new();
            Resampler::new(rate).process(&input, &mut whole);
            let mut streaming = Vec::new();
            let mut converter = Resampler::new(rate);
            for chunk in input.chunks(137) {
                converter.process(chunk, &mut streaming);
            }
            assert_eq!(whole.len(), rate as usize);
            assert_eq!(whole, streaming);
            converter.reset();
            let mut restarted = Vec::new();
            converter.process(&input, &mut restarted);
            assert_eq!(restarted, whole);
        }
    }
}
