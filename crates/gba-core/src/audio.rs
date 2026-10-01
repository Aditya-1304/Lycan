//! Timers, Direct Sound FIFOs and the stereo PWM mixer on the machine clock.
//! Host adapters consume fixed-rate frames and never own device registers.

use crate::{pulse::Pulse, wave::Wave};
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
    pulses: [Pulse; 2],
    wave: Wave,
    next_sequence: u64,
    sequence_step: u8,
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
            pulses: std::array::from_fn(|_| Pulse::default()),
            wave: Wave::default(),
            next_sequence: 32768,
            sequence_step: 0,
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
            0x90..=0x9e => self.wave.write(offset, value),
            0x70 | 0x72 | 0x74 if self.master => self.wave.write(offset, value),
            0x60 | 0x62 | 0x64 | 0x68 | 0x6c if self.master => {
                let (index, register) = match offset {
                    0x60 => (0, 0),
                    0x62 => (0, 1),
                    0x64 => (0, 2),
                    0x68 => (1, 1),
                    _ => (1, 2),
                };
                self.pulses[index].write(register, value, index == 0);
            }
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
                    self.pulses = std::array::from_fn(|_| Pulse::default());
                    self.wave.reset();
                    self.sequence_step = 0;
                }
            }
            0x88 => self.bias = value & 0xc3fe,
            _ => {}
        }
    }

    /// Byte writes merge against writable latches, never masked readback. In
    /// particular, a high-byte trigger must retain the write-only frequency low byte.
    pub fn pulse_latch(&self, offset: usize) -> Option<u16> {
        match offset {
            0x70 => Some(self.wave.select),
            0x72 => Some(self.wave.volume),
            0x74 => Some(self.wave.control),
            0x60 => Some(self.pulses[0].sweep),
            0x62 => Some(self.pulses[0].envelope),
            0x64 => Some(self.pulses[0].control),
            0x68 => Some(self.pulses[1].envelope),
            0x6c => Some(self.pulses[1].control),
            _ => None,
        }
    }

    /// Exposes readable PSG fields and activity flags; trigger and frequency
    /// fields are write-only. Direct Sound does not set PSG activity flags.
    pub fn refresh_controls(&self, io: &mut [u8]) {
        self.wave.refresh(io);
        for (offset, value) in [
            (0x60, self.pulses[0].sweep),
            (0x62, self.pulses[0].envelope & 0xffc0),
            (0x64, self.pulses[0].control & 0x4000),
            (0x68, self.pulses[1].envelope & 0xffc0),
            (0x6c, self.pulses[1].control & 0x4000),
            (0x80, self.control_l),
            (0x82, self.control_h),
            (
                0x84,
                (u16::from(self.master) << 7)
                    | u16::from(self.pulses[0].active)
                    | (u16::from(self.pulses[1].active) << 1)
                    | (u16::from(self.wave.active) << 2),
            ),
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
        let mut next = self
            .next_sample
            .min((now / period + 1) * period)
            .min(self.next_sequence);
        if let Some(edge) = self.wave.next_edge() {
            next = next.min(now + edge);
        }
        for pulse in &self.pulses {
            if let Some(edge) = pulse.next_edge() {
                next = next.min(now + edge);
            }
        }
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
        self.wave.advance(elapsed);
        for pulse in &mut self.pulses {
            pulse.advance(elapsed);
        }
        if now == self.next_sequence {
            if self.master {
                for (index, pulse) in self.pulses.iter_mut().enumerate() {
                    pulse.sequence(self.sequence_step, index == 0);
                }
                self.wave.sequence(self.sequence_step);
                self.sequence_step = (self.sequence_step + 1) & 7;
            }
            self.next_sequence += 32768;
        }
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
        let ratio = [1, 2, 4, 4][(self.control_h & 3) as usize];
        for (channel, signal) in [
            self.pulses[0].level(),
            self.pulses[1].level(),
            self.wave.level(),
        ]
        .into_iter()
        .enumerate()
        {
            for (side, level) in levels.iter_mut().enumerate() {
                let route = if side == 0 { 12 } else { 8 };
                let volume = if side == 0 {
                    (self.control_l >> 4) & 7
                } else {
                    self.control_l & 7
                };
                if self.control_l & (1 << (route + channel)) != 0 {
                    *level += signal * i32::from(volume + 1) * ratio / 4;
                }
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

    // Pulse/FIFO tests cannot detect missing wave synthesis or writes reaching
    // the playing bank. Exercise the public register and PCM paths together.
    #[test]
    fn wave_bank_access_and_streaming_signal() {
        fn render(chunk: u64) -> Vec<[f32; 2]> {
            let mut audio = Audio::new();
            audio.write_control(0x84, 0x80);
            audio.write_control(0x80, 0x4477);
            audio.write_control(0x82, 2);
            audio.write_control(0x70, 0x40);
            for offset in (0x90..0xa0).step_by(2) {
                audio.write_control(offset, 0xffff);
            }
            audio.write_control(0x70, 0);
            for offset in (0x90..0xa0).step_by(2) {
                audio.write_control(offset, 0);
            }
            audio.write_control(0x70, 0xa0);
            audio.write_control(0x72, 0x2000);
            audio.write_control(0x74, 0x87c0);
            let mut output = Vec::new();
            let mut now = 0;
            while now < 16384 {
                let next = audio.next_event(now);
                audio.advance(next - now, next);
                now = next;
                if now % chunk == 0 {
                    audio.drain_stereo(&mut output);
                }
            }
            audio.drain_stereo(&mut output);
            assert!(output.iter().any(|f| f[0] > 0.0), "bank zero must play");
            assert!(
                output.iter().any(|f| f[0] < 0.0),
                "64-sample playback must reach bank one"
            );
            output
        }
        assert_eq!(render(512), render(8192));
        let mut audio = Audio::new();
        audio.write_control(0x84, 0x80);
        audio.write_control(0x70, 0x40);
        audio.write_control(0x90, 0xffff);
        audio.write_control(0x70, 0x80);
        audio.write_control(0x72, 0x20ff);
        audio.write_control(0x74, 0xc700);
        assert_eq!(audio.wave.level(), 15);
        audio.write_control(0x72, 0x80ff);
        assert_eq!(audio.wave.level(), 11, "forced 75% ignores volume code");
        audio.write_control(0x72, 0x40ff);
        assert_eq!(audio.wave.level(), 7);
        audio.write_control(0x72, 0x60ff);
        assert_eq!(audio.wave.level(), 3);
        audio.wave.sequence(0);
        assert!(!audio.wave.active, "one length tick expires 256-255");
        audio.write_control(0x74, 0x8700);
        assert!(audio.wave.active);
        audio.write_control(0x70, 0);
        assert!(!audio.wave.active, "DAC disable stops playback");
        audio.write_control(0x84, 0);
        audio.write_control(0x70, 0x80);
        audio.write_control(0x74, 0x8700);
        assert!(!audio.wave.active, "master-disabled PSG writes are ignored");
        let mut io = vec![0; 0xa0];
        audio.refresh_controls(&mut io);
        assert_eq!(&io[0x90..0x92], &[0, 0]);
        audio.write_control(0x84, 0x80);
        audio.write_control(0x70, 0x40);
        audio.refresh_controls(&mut io);
        assert_eq!(&io[0x90..0x92], &[255, 255], "wave RAM survives PSG reset");
    }

    // FIFO regressions cannot detect absent PSG synthesis or a drain resetting
    // oscillator history. Compare actual mixed samples across two drain schedules.
    #[test]
    fn pulse_voices_mix_with_fifo_and_preserve_chunk_history() {
        fn render(chunk: u64) -> Vec<[f32; 2]> {
            let mut audio = Audio::new();
            audio.write_control(0x84, 0x80);
            audio.write_control(0x80, 0x2177);
            audio.write_control(0x82, 0x0306);
            audio.push_channel(0, &[16; 32]);
            audio.write_timer(0x100, 0xfe00);
            audio.write_timer(0x102, 0x80);
            audio.write_control(0x60, 0x29);
            audio.write_control(0x62, 0xa180);
            audio.write_control(0x64, 0x8400);
            audio.write_control(0x68, 0x6880);
            audio.write_control(0x6c, 0x8600);
            let mut output = Vec::new();
            let mut now = 0;
            while now < 524_288 {
                let next = audio.next_event(now);
                audio.advance(next - now, next);
                now = next;
                if now % chunk == 0 {
                    audio.drain_stereo(&mut output);
                }
            }
            audio.drain_stereo(&mut output);
            assert_eq!(
                audio.pulses[0].control & 0x7ff,
                256,
                "two decreasing sweep updates"
            );
            assert_eq!(audio.pulses[0].level().abs(), 8, "two envelope steps");
            assert!(
                output.iter().any(|f| f[0] != f[1]),
                "second voice must be independently routed"
            );
            assert!(
                output.iter().any(|f| f[0].abs() > 0.125),
                "pulse must mix with FIFO"
            );
            output
        }
        assert_eq!(render(512), render(8192));
    }

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
        let mut io = vec![0; 0xa0];
        audio.refresh_controls(&mut io);
        assert_eq!(io[0x84], 0); // Direct Sound never sets PSG status flags.
        audio.write_control(0x84, 0x80);
        audio.refresh_controls(&mut io);
        assert_eq!(io[0x84], 0x80);
    }
}
