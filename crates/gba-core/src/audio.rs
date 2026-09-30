//! Timers, Direct Sound FIFOs and the stereo PWM mixer on the machine clock.
//! Host adapters consume fixed-rate frames and never own device registers.

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
    fifo: [VecDeque<i8>; 2],
    held: [i8; 2],
    control_l: u16,
    control_h: u16,
    master: bool,
    bias: u16,
    pwm_sum: [f32; 2],
    pwm_samples: u32,
    last_pwm: u64,
    next_sample: u64,
    pcm: VecDeque<[f32; 2]>,
    pub dropped: u64,
    pub fifo_underruns: u64,
    pub produced: u64,
}

impl Audio {
    pub fn new() -> Self {
        Self {
            timers: [Timer::default(); 4],
            fifo: std::array::from_fn(|_| VecDeque::with_capacity(32)),
            held: [0; 2],
            control_l: 0,
            control_h: 0,
            master: false,
            bias: 0x200,
            pwm_sum: [0.0; 2],
            pwm_samples: 0,
            last_pwm: 0,
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

    /// Applies writable masks and self-clearing FIFO reset strobes. Master
    /// disable resets PSG control storage, but preserves Direct Sound devices.
    pub fn write_control(&mut self, offset: usize, value: u16) {
        match offset {
            0x80 => {
                if self.master {
                    self.control_l = value & 0xff77;
                }
            }
            0x82 => {
                self.control_h = value & 0x770f;
                for channel in 0..2 {
                    if value & (0x800 << (channel * 4)) != 0 {
                        self.fifo[channel].clear();
                        self.held[channel] = 0;
                    }
                }
            }
            0x84 => {
                self.master = value & 0x80 != 0;
                if !self.master {
                    self.control_l = 0;
                }
            }
            0x88 => self.bias = value & 0xc3fe,
            _ => {}
        }
    }

    /// PSG status bits remain zero until their channels are implemented; FIFO
    /// playback never sets the four PSG activity flags in SOUNDCNT_X.
    pub fn refresh_controls(&self, io: &mut [u8]) {
        for (offset, value) in [
            (0x80, self.control_l),
            (0x82, self.control_h),
            (0x84, u16::from(self.master) << 7),
            (0x88, self.bias),
        ] {
            io[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }
    }

    /// FIFO bus writes append signed bytes in little-endian order and discard
    /// overflow beyond the hardware's 32-byte capacity.
    pub fn push_channel(&mut self, channel: usize, bytes: &[u8]) {
        for &byte in bytes {
            if self.fifo[channel].len() < 32 {
                self.fifo[channel].push_back(byte as i8);
            }
        }
    }

    /// Stop synchronization at the earliest sample or timer edge. Cascaded
    /// timers advance from the preceding overflow, never from host elapsed time.
    pub fn next_event(&self, now: u64) -> u64 {
        let period = SAMPLE_CYCLES >> (self.bias >> 14);
        let mut next = self.next_sample.min((now / period + 1) * period);
        for (index, timer) in self.timers.iter().enumerate() {
            if timer.enabled() && !timer.cascade(index) {
                next = next
                    .min(now + (65536 - u64::from(timer.counter)) * timer.period() - timer.phase);
            }
        }
        next
    }

    /// Called only at event boundaries, so each timer can overflow at most once.
    /// Each FIFO consumes its selected timer, independently of master enable.
    /// Returns ordinary timer IF bits and the FIFO DMA request level.
    pub fn advance(&mut self, elapsed: u64, now: u64) -> (u16, [bool; 2]) {
        let mut previous_overflow = false;
        let mut irq = 0;
        let mut refill = [false; 2];
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
                for (channel, request) in refill.iter_mut().enumerate() {
                    let shift = channel * 4;
                    let selected = usize::from(self.control_h & (0x400 << shift) != 0);
                    if index == selected {
                        if let Some(sample) = self.fifo[channel].pop_front() {
                            self.held[channel] = sample;
                        } else if self.control_h & (0x300 << shift) != 0 {
                            self.fifo_underruns += 1;
                        }
                        *request = self.fifo[channel].len() <= 16;
                    }
                }
            } else {
                timer.counter = value as u16;
            }
        }
        let period = SAMPLE_CYCLES >> (self.bias >> 14);
        if now > self.last_pwm && now.is_multiple_of(period) {
            self.last_pwm = now;
            let frame = self.mix();
            for (sum, value) in self.pwm_sum.iter_mut().zip(frame) {
                *sum += value;
            }
            self.pwm_samples += 1;
        }
        if now == self.next_sample {
            if self.pcm.len() == PCM_CAPACITY {
                self.pcm.pop_front();
                self.dropped += 1;
            }
            let count = self.pwm_samples.max(1) as f32;
            self.pcm.push_back(if self.master {
                self.pwm_sum.map(|sum| sum / count)
            } else {
                [0.0; 2]
            });
            self.pwm_sum = [0.0; 2];
            self.pwm_samples = 0;
            self.produced += 1;
            self.next_sample += SAMPLE_CYCLES;
        }
        (irq, refill)
    }

    pub fn drain(&mut self, output: &mut Vec<f32>) {
        output.extend(self.pcm.drain(..).map(|frame| (frame[0] + frame[1]) * 0.5));
    }

    /// Drains left/right frames atomically so queue depth remains measured in
    /// frames throughout core, resampling and platform output.
    pub fn drain_stereo(&mut self, output: &mut Vec<[f32; 2]>) {
        output.extend(self.pcm.drain(..));
    }

    /// Applies Direct Sound gain, routing, unsigned DAC saturation and PWM
    /// resolution. The fixed 32768 Hz stream averages higher-rate PWM edges;
    /// it models audible DAC levels rather than the MHz carrier waveform.
    fn mix(&self) -> [f32; 2] {
        if !self.master {
            return [0.0; 2];
        }
        let mut levels = [0i32; 2];
        for channel in 0..2 {
            let shift = channel * 4;
            let gain = if self.control_h & (4 << channel) != 0 {
                4
            } else {
                2
            };
            let value = i32::from(self.held[channel]) * gain;
            if self.control_h & (0x200 << shift) != 0 {
                levels[0] += value;
            }
            if self.control_h & (0x100 << shift) != 0 {
                levels[1] += value;
            }
        }
        let mask = !((1 << (1 + (self.bias >> 14))) - 1);
        levels.map(|level| {
            let dac = (level + i32::from(self.bias & 0x3fe)).clamp(0, 1023) & mask;
            (dac - 512) as f32 / 512.0
        })
    }

    pub fn clear_pcm(&mut self) {
        self.pcm.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Existing DMA checks do not detect master-disable writes being ignored.
    // The FIFO must keep consuming bytes while the mixer emits silence.
    #[test]
    fn disabled_master_silences_output_without_stopping_fifo() {
        let mut audio = Audio::new();
        audio.push_channel(0, &[32, 64]);
        audio.write_timer(0x100, 0xfe00);
        audio.write_timer(0x102, 0x80);
        audio.advance(512, 512);
        let mut samples = Vec::new();
        audio.drain(&mut samples);
        assert_eq!(samples, [0.0]);
        assert_eq!(audio.held[0], 32);
        assert_eq!(audio.fifo[0].len(), 1);

        audio.write_control(0x84, 0xffff);
        audio.write_control(0x80, 0xffff);
        assert_eq!(audio.control_l, 0xff77);
        // A uses timer 0 at half gain on the left. B uses timer 1 at full
        // gain on the right, so timer 0 must not consume B prematurely.
        audio.write_control(0x82, 0x5208);
        audio.push_channel(1, &[65, 191]);
        audio.write_timer(0x104, 0xfc00);
        audio.write_timer(0x106, 0x80);
        audio.advance(512, 1024);
        assert_eq!(audio.held, [64, 0]);
        audio.advance(512, 1536);
        assert_eq!(audio.held, [64, 65]);
        assert_eq!(audio.mix(), [0.25, 260.0 / 512.0]);
        for (mode, right) in [(0, 260), (1, 260), (2, 256), (3, 256)] {
            audio.write_control(0x88, 0x200 | (mode << 14));
            assert_eq!(audio.mix(), [0.25, right as f32 / 512.0]);
        }
        audio.write_control(0x88, 0xffff);
        assert_eq!(audio.bias, 0xc3fe);
        assert_eq!(audio.mix(), [496.0 / 512.0; 2]);
        audio.write_control(0x88, 0x200);
        audio.write_control(0x82, 0xd208);
        assert_eq!(audio.control_h, 0x5208);
        assert!(audio.fifo[1].is_empty());
        assert_eq!(audio.held, [64, 0]);
        let timers = audio.timers.map(|timer| (timer.counter, timer.phase));
        audio.write_control(0x84, 0);
        assert_eq!(audio.control_l, 0);
        assert_eq!(audio.mix(), [0.0; 2]);
        assert_eq!(
            audio.timers.map(|timer| (timer.counter, timer.phase)),
            timers
        );
        assert_eq!(audio.held[0], 64);
        let mut io = vec![0; 0x90];
        audio.refresh_controls(&mut io);
        assert_eq!(io[0x84], 0); // Direct Sound never sets PSG status flags.
        audio.write_control(0x84, 0x80);
        audio.refresh_controls(&mut io);
        assert_eq!(io[0x84], 0x80);
    }
}
