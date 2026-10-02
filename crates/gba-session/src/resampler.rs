//! Streaming conversion from the fixed core cadence to a host device rate.

use crate::PCM_RATE;
use std::num::NonZeroU32;

/// Linear interpolation with an integer phase carried across chunk boundaries.
/// Reset only on session discontinuities; ordinary frame boundaries retain it.
pub struct Resampler {
    output_rate: u32,
    previous: Option<f32>,
    phase: u64,
}

impl Resampler {
    /// Output adapters supply their actual device/AudioContext sample rate.
    pub fn new(output_rate: NonZeroU32) -> Self {
        Self {
            output_rate: output_rate.get(),
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

/// Linear stereo interpolation with one phase shared by both channels.
/// Reset only on session discontinuities; ordinary frame boundaries retain it.
pub struct StereoResampler {
    output_rate: u32,
    previous: Option<[f32; 2]>,
    phase: u64,
}

impl StereoResampler {
    /// Output adapters supply their actual device/AudioContext sample rate.
    pub fn new(output_rate: NonZeroU32) -> Self {
        Self {
            output_rate: output_rate.get(),
            previous: None,
            phase: 0,
        }
    }

    /// Appends stereo samples while preserving phase across input chunks.
    pub fn process(&mut self, input: &[[f32; 2]], output: &mut Vec<[f32; 2]>) {
        for &sample in input {
            if let Some(previous) = self.previous {
                while self.phase < u64::from(self.output_rate) {
                    let fraction = self.phase as f32 / self.output_rate as f32;

                    output.push([
                        previous[0] + (sample[0] - previous[0]) * fraction,
                        previous[1] + (sample[1] - previous[1]) * fraction,
                    ]);

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
            let Some(output_rate) = NonZeroU32::new(rate) else {
                continue;
            };

            let mut whole = Vec::new();
            Resampler::new(output_rate).process(&input, &mut whole);
            let mut streaming = Vec::new();
            let mut converter = Resampler::new(output_rate);
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

    #[test]
    fn stereo_resampling_matches_independent_mono_resamplers() {
        let input: Vec<_> = (0..=PCM_RATE)
            .map(|index| {
                [
                    ((index * 17) % 251) as f32 / 251.0,
                    ((index * 31 + 7) % 263) as f32 / 263.0,
                ]
            })
            .collect();

        for rate in [44_100, 48_000, 96_000] {
            let Some(output_rate) = NonZeroU32::new(rate) else {
                continue;
            };

            let mut stereo_converter = super::StereoResampler::new(output_rate);
            let mut stereo = Vec::new();
            let mut left_converter = Resampler::new(output_rate);
            let mut right_converter = Resampler::new(output_rate);
            let mut left = Vec::new();
            let mut right = Vec::new();

            for chunk in input.chunks(137) {
                let left_chunk: Vec<_> = chunk.iter().map(|frame| frame[0]).collect();
                let right_chunk: Vec<_> = chunk.iter().map(|frame| frame[1]).collect();

                stereo_converter.process(chunk, &mut stereo);
                left_converter.process(&left_chunk, &mut left);
                right_converter.process(&right_chunk, &mut right);
            }

            assert_eq!(
                left.len(),
                right.len(),
                "channel lengths differ at {rate} Hz"
            );
            assert_eq!(
                stereo.len(),
                left.len(),
                "stereo length differs at {rate} Hz"
            );
            assert_eq!(
                stereo,
                left.iter()
                    .zip(&right)
                    .map(|(&left, &right)| [left, right])
                    .collect::<Vec<_>>(),
                "stereo samples differ from independent mono output at {rate} Hz"
            );
        }
    }
}
