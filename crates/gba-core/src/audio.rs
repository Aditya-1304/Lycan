//! Timer and Direct Sound A state on the machine clock. Host playback owns no
//! device registers. Mixer routing, channel B and PWM controls belong to 12A.

use std::collections::VecDeque;

/// Fixed core output cadence; host adapters resample this stream continuously.
pub const PCM_RATE: u32 = 32_768;
const SAMPLE_CYCLES: u64 = 512;
const PCM_CAPACITY: usize = 4096;

#[derive(Clone, Copy, Default)]
struct Timer {
    reload: u16,
    counter: u16,
    control: u16,
    phase: u64,
}

impl Timer {
    fn period(self) -> u64 {
        [1, 64, 256, 1024][(self.control & 3) as usize]
    }

    fn enabled(self) -> bool {
        self.control & 0x80 != 0
    }

    fn cascade(self, index: usize) -> bool {
        index != 0 && self.control & 4 != 0
    }
}

/// Bounded guest PCM staging and hardware FIFO. All storage is reserved once,
/// never allocated per sample; a consumer that stops draining cannot grow it.
pub(super) struct Audio {
    timers: [Timer; 4],
    fifo: VecDeque<i8>,
    held: i8,
    next_sample: u64,
    pcm: VecDeque<f32>,
    pub dropped: u64,
    pub fifo_underruns: u64,
    pub produced: u64,
}

impl Audio {
    pub fn new() -> Self {
        Self {
            timers: [Timer::default(); 4],
            fifo: VecDeque::with_capacity(32),
            held: 0,
            next_sample: SAMPLE_CYCLES,
            pcm: VecDeque::with_capacity(PCM_CAPACITY),
            dropped: 0,
            fifo_underruns: 0,
            produced: 0,
        }
    }

    /// Reload writes do not change a running counter. The enable edge reloads
    /// and reanchors the prescaler; disabling preserves the readable count.
    pub fn write_timer(&mut self, offset: usize, value: u16) {
        let index = (offset - 0x100) / 4;
        let timer = &mut self.timers[index];
        if offset & 2 == 0 {
            timer.reload = value;
        } else {
            if value & 0x80 != 0 && !timer.enabled() {
                timer.counter = timer.reload;
                timer.phase = 0;
            }
            timer.control = value & if index == 0 { 0xc3 } else { 0xc7 };
            // A changed divider must retain a valid fractional tick, even when
            // software reprograms an enabled timer to a faster clock.
            timer.phase %= timer.period();
        }
    }

    pub fn refresh_timers(&self, io: &mut [u8]) {
        for (index, timer) in self.timers.iter().enumerate() {
            let offset = 0x100 + index * 4;
            io[offset..offset + 2].copy_from_slice(&timer.counter.to_le_bytes());
            io[offset + 2..offset + 4].copy_from_slice(&timer.control.to_le_bytes());
        }
    }

    pub fn push(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if self.fifo.len() < 32 {
                self.fifo.push_back(byte as i8);
            }
        }
    }

    /// Stop synchronization at the earliest sample or timer edge. Cascaded
    /// timers advance from the preceding overflow, never from host elapsed time.
    pub fn next_event(&self, now: u64) -> u64 {
        let mut next = self.next_sample;
        for (index, timer) in self.timers.iter().enumerate() {
            if timer.enabled() && !timer.cascade(index) {
                next = next
                    .min(now + (65536 - u64::from(timer.counter)) * timer.period() - timer.phase);
            }
        }
        next
    }

    /// Called only at event boundaries, so each timer can overflow at most once.
    /// Timer 0 consumes A; the resulting level is sampled at the fixed PCM edge.
    /// Returns ordinary timer IF bits and the FIFO DMA request level.
    pub fn advance(&mut self, elapsed: u64, now: u64) -> (u16, bool) {
        let mut previous_overflow = false;
        let mut irq = 0;
        let mut refill = false;
        for (index, timer) in self.timers.iter_mut().enumerate() {
            let ticks = if !timer.enabled() {
                0
            } else if timer.cascade(index) {
                u64::from(previous_overflow)
            } else {
                let total = timer.phase + elapsed;
                timer.phase = total % timer.period();
                total / timer.period()
            };
            let value = u64::from(timer.counter) + ticks;
            previous_overflow = value >= 65536;
            if previous_overflow {
                timer.counter = timer.reload;
                if timer.control & 0x40 != 0 {
                    irq |= 1 << (3 + index);
                }
                if index == 0 {
                    if let Some(sample) = self.fifo.pop_front() {
                        self.held = sample;
                    } else {
                        self.fifo_underruns += 1;
                    }
                    refill = self.fifo.len() <= 16;
                }
            } else {
                timer.counter = value as u16;
            }
        }
        if now == self.next_sample {
            if self.pcm.len() == PCM_CAPACITY {
                self.pcm.pop_front();
                self.dropped += 1;
            }
            self.pcm.push_back(f32::from(self.held) / 128.0);
            self.produced += 1;
            self.next_sample += SAMPLE_CYCLES;
        }
        (irq, refill)
    }

    pub fn drain(&mut self, output: &mut Vec<f32>) {
        output.extend(self.pcm.drain(..));
    }

    pub fn clear_pcm(&mut self) {
        self.pcm.clear();
    }
}
