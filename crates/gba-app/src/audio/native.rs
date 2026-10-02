//! CPAL consumes a bounded SPSC queue; the callback never runs the machine,
//! locks, logs, or allocates. Epoch tags invalidate sound from old sessions.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};

#[derive(Default)]
struct Shared {
    epoch: AtomicU64,
    active: AtomicBool,
    gain: AtomicU32,
    underruns: AtomicU64,
    underrun_events: AtomicU64,
    callbacks: AtomicU64,
    max_callback_frames: AtomicU64,
    played: AtomicU64,
    errors: AtomicU64,
}

/// Owns the live CPAL stream and its single frontend queue producer.
pub struct Output {
    // Keeping the stream alive owns the callback and its consumer.
    _stream: cpal::Stream,
    producer: Producer<(u64, [f32; 2])>,
    shared: Arc<Shared>,
    rate: u32,
    capacity: usize,
    overflows: u64,
    overflow_events: u64,
    max_depth: usize,
}

impl Output {
    pub fn start() -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("No audio output device")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let config = supported.config();
        let rate = config.sample_rate;
        let capacity = (rate as usize * 80 / 1000).max(1);
        let (producer, consumer) = RingBuffer::new(capacity);
        let shared = Arc::new(Shared::default());
        shared.gain.store(0.5_f32.to_bits(), Ordering::Relaxed);
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => {
                stream::<f32>(&device, &config, consumer, Arc::clone(&shared))
            }
            cpal::SampleFormat::I16 => {
                stream::<i16>(&device, &config, consumer, Arc::clone(&shared))
            }
            cpal::SampleFormat::U16 => {
                stream::<u16>(&device, &config, consumer, Arc::clone(&shared))
            }
            format => return Err(format!("Unsupported audio format: {format}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            _stream: stream,
            producer,
            shared,
            rate,
            capacity,
            overflows: 0,
            overflow_events: 0,
            max_depth: 0,
        })
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn submit(&mut self, samples: &[[f32; 2]]) {
        if !self.shared.active.load(Ordering::Relaxed) {
            return;
        }
        let epoch = self.shared.epoch.load(Ordering::Acquire);
        let accepted = self.producer.slots().min(samples.len());

        let written = if accepted == 0 {
            0
        } else {
            match self.producer.write_chunk_uninit(accepted) {
                Ok(chunk) => chunk.fill_from_iter(
                    samples[..accepted]
                        .iter()
                        .copied()
                        .map(|sample| (epoch, sample)),
                ),
                Err(_) => 0,
            }
        };

        let dropped = samples.len() - written;
        if dropped != 0 {
            self.overflows += dropped as u64;
            self.overflow_events += 1;
        }
        self.max_depth = self.max_depth.max(self.capacity - self.producer.slots());
    }

    pub fn clear(&mut self) {
        self.shared.epoch.fetch_add(1, Ordering::AcqRel);
    }

    pub fn set_playing(&mut self, playing: bool) {
        if self.shared.active.swap(playing, Ordering::AcqRel) != playing {
            self.clear();
        }
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.shared.gain.store(gain.to_bits(), Ordering::Relaxed);
    }

    pub fn status(&self) -> String {
        format!(
            "{} Hz | queue {:.1} ms (max {:.1}, cap 80) | underrun events {} frames {} | overflow events {} frames {} | callbacks {} max {} frames | output frames {} | device errors {}",
            self.rate,
            (self.capacity - self.producer.slots()) as f64 * 1000.0 / f64::from(self.rate),
            self.max_depth as f64 * 1000.0 / f64::from(self.rate),
            self.shared.underrun_events.load(Ordering::Relaxed),
            self.shared.underruns.load(Ordering::Relaxed),
            self.overflow_events,
            self.overflows,
            self.shared.callbacks.load(Ordering::Relaxed),
            self.shared.max_callback_frames.load(Ordering::Relaxed),
            self.shared.played.load(Ordering::Relaxed),
            self.shared.errors.load(Ordering::Relaxed)
        )
    }
}

fn stream<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut consumer: Consumer<(u64, [f32; 2])>,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, cpal::Error> {
    let channels = usize::from(config.channels);
    // Prime once per lifecycle epoch; half the queue remains available for
    // catch-up bursts while the buffered half absorbs producer jitter.
    let target = (config.sample_rate as usize * 40 / 1000).max(1);
    let errors = Arc::clone(&shared);
    let mut playback = Playback::default();
    device.build_output_stream(
        *config,
        move |data: &mut [T], _| {
            playback.fill(data, channels, target, &mut consumer, &shared);
        },
        move |_| {
            errors.errors.fetch_add(1, Ordering::Relaxed);
        },
        None,
    )
}

/// Consumer state belongs exclusively to the audio callback. Extracting it from
/// stream creation allows recovery to be verified without a physical device.
#[derive(Default)]
struct Playback {
    epoch: u64,
    primed: bool,
}

impl Playback {
    fn fill<T: cpal::SizedSample + cpal::FromSample<f32>>(
        &mut self,
        data: &mut [T],
        channels: usize,
        target: usize,
        consumer: &mut Consumer<(u64, [f32; 2])>,
        shared: &Shared,
    ) {
        shared.callbacks.fetch_add(1, Ordering::Relaxed);
        shared
            .max_callback_frames
            .fetch_max((data.len() / channels) as u64, Ordering::Relaxed);
        let current = shared.epoch.load(Ordering::Acquire);
        if current != self.epoch {
            self.epoch = current;
            self.primed = false;
        }
        // Retain samples submitted after the boundary while discarding old epochs.
        while consumer.peek().is_ok_and(|sample| sample.0 != self.epoch) {
            let _ = consumer.pop();
        }
        let active = shared.active.load(Ordering::Acquire);
        if active && !self.primed && consumer.slots() >= target {
            self.primed = true;
        }
        let gain = f32::from_bits(shared.gain.load(Ordering::Relaxed));
        let mut missing = 0;
        let mut played = 0;
        for frame in data.chunks_mut(channels) {
            let sample = if active && self.primed {
                match consumer.pop() {
                    Ok((tag, sample)) if tag == self.epoch => {
                        played += 1;
                        sample.map(|value| value * gain)
                    }
                    _ => {
                        missing += 1;
                        [0.0; 2]
                    }
                }
            } else {
                [0.0; 2]
            };
            // Mono devices receive a downmix; stereo devices preserve the
            // guest routing. Extra surround channels remain silent.
            for (index, output) in frame.iter_mut().enumerate() {
                let value = if channels == 1 {
                    (sample[0] + sample[1]) * 0.5
                } else {
                    sample.get(index).copied().unwrap_or(0.0)
                };
                *output = T::from_sample(value);
            }
        }
        // A transient shortage silences only missing samples. Repriming
        // here would amplify a short scheduling miss into startup latency.
        if missing != 0 {
            shared.underrun_events.fetch_add(1, Ordering::Relaxed);
        }
        shared.underruns.fetch_add(missing, Ordering::Relaxed);
        shared.played.fetch_add(played, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A short underrun must not turn into a second startup silence while the
    /// next samples are already available. Resampler tests do not cover CPAL's
    /// queue consumer or its priming state.
    #[test]
    fn transient_underrun_resumes_without_repriming() {
        let (mut producer, mut consumer) = RingBuffer::new(8);
        let shared = Shared::default();
        shared.active.store(true, Ordering::Relaxed);
        shared.gain.store(1.0_f32.to_bits(), Ordering::Relaxed);
        let mut playback = Playback::default();
        for _ in 0..4 {
            producer.push((0, [0.5; 2])).unwrap();
        }
        let mut output = [0.0_f32; 12];
        playback.fill(&mut output, 2, 4, &mut consumer, &shared);
        assert_eq!(&output[..8], &[0.5; 8]);
        assert_eq!(&output[8..], &[0.0; 4]);
        producer.push((0, [0.25; 2])).unwrap();
        let mut resumed = [0.0_f32; 2];
        playback.fill(&mut resumed, 2, 4, &mut consumer, &shared);
        assert_eq!(resumed, [0.25; 2]);
        assert_eq!(shared.underruns.load(Ordering::Relaxed), 2);
        assert_eq!(shared.underrun_events.load(Ordering::Relaxed), 1);
        assert_eq!(shared.callbacks.load(Ordering::Relaxed), 2);
        assert_eq!(shared.max_callback_frames.load(Ordering::Relaxed), 6);

        // A lifecycle boundary still invalidates queued sound and requires
        // startup priming for the new epoch.
        producer.push((0, [0.75; 2])).unwrap();
        shared.epoch.store(1, Ordering::Release);
        producer.push((1, [0.125; 2])).unwrap();
        playback.fill(&mut resumed, 2, 4, &mut consumer, &shared);
        assert_eq!(resumed, [0.0; 2]);
        for _ in 0..3 {
            producer.push((1, [0.125; 2])).unwrap();
        }
        playback.fill(&mut resumed, 2, 4, &mut consumer, &shared);
        assert_eq!(resumed, [0.125; 2]);
    }
}
