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
    played: AtomicU64,
    errors: AtomicU64,
}

/// Owns the live CPAL stream and its single frontend queue producer.
pub struct Output {
    // Keeping the stream alive owns the callback and its consumer.
    _stream: cpal::Stream,
    producer: Producer<(u64, f32)>,
    shared: Arc<Shared>,
    rate: u32,
    capacity: usize,
    overflows: u64,
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
            cpal::SampleFormat::F32 => stream::<f32>(&device, &config, consumer, shared.clone()),
            cpal::SampleFormat::I16 => stream::<i16>(&device, &config, consumer, shared.clone()),
            cpal::SampleFormat::U16 => stream::<u16>(&device, &config, consumer, shared.clone()),
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
            max_depth: 0,
        })
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn submit(&mut self, samples: &[f32]) {
        if !self.shared.active.load(Ordering::Relaxed) {
            return;
        }
        let epoch = self.shared.epoch.load(Ordering::Acquire);
        for &sample in samples {
            if self.producer.push((epoch, sample)).is_err() {
                self.overflows += 1;
            }
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
            "{} Hz | queue {:.1} ms (max {:.1}, cap 80) | underrun frames {} | overflow frames {} | output frames {} | device errors {}",
            self.rate,
            (self.capacity - self.producer.slots()) as f64 * 1000.0 / f64::from(self.rate),
            self.max_depth as f64 * 1000.0 / f64::from(self.rate),
            self.shared.underruns.load(Ordering::Relaxed),
            self.overflows,
            self.shared.played.load(Ordering::Relaxed),
            self.shared.errors.load(Ordering::Relaxed)
        )
    }
}

fn stream<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut consumer: Consumer<(u64, f32)>,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, cpal::Error> {
    let channels = usize::from(config.channels);
    let target = config.sample_rate as usize * 60 / 1000;
    let errors = shared.clone();
    let mut epoch = 0;
    let mut primed = false;
    device.build_output_stream(
        *config,
        move |data: &mut [T], _| {
            let current = shared.epoch.load(Ordering::Acquire);
            if current != epoch {
                epoch = current;
                primed = false;
            }
            // Retain samples submitted after the boundary while discarding old epochs.
            while consumer.peek().is_ok_and(|sample| sample.0 != epoch) {
                let _ = consumer.pop();
            }
            let active = shared.active.load(Ordering::Acquire);
            if active && !primed && consumer.slots() >= target {
                primed = true;
            }
            let gain = f32::from_bits(shared.gain.load(Ordering::Relaxed));
            let mut missing = 0;
            let mut played = 0;
            for frame in data.chunks_mut(channels) {
                let sample = if active && primed {
                    match consumer.pop() {
                        Ok((tag, sample)) if tag == epoch => {
                            played += 1;
                            sample * gain
                        }
                        _ => {
                            missing += 1;
                            0.0
                        }
                    }
                } else {
                    0.0
                };
                frame.fill(T::from_sample(sample));
            }
            if missing != 0 {
                primed = false;
            }
            shared.underruns.fetch_add(missing, Ordering::Relaxed);
            shared.played.fetch_add(played, Ordering::Relaxed);
        },
        move |_| {
            errors.errors.fetch_add(1, Ordering::Relaxed);
        },
        None,
    )
}
