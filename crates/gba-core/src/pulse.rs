//! Pulse oscillators clocked by the GBA master clock and 512 Hz PSG sequencer.
//! Channel state belongs to the machine; draining PCM never resets this state.

/// One eight-step oscillator with length, envelope and optional frequency sweep.
#[derive(Default)]
pub(super) struct Pulse {
    pub sweep: u16,
    pub envelope: u16,
    pub control: u16,
    pub active: bool,
    frequency: u16,
    phase: u64,
    step: usize,
    length: u8,
    volume: u8,
    envelope_ticks: u8,
    sweep_ticks: u8,
    sweep_shadow: u16,
    sweep_enabled: bool,
    swept_down: bool,
}

impl Pulse {
    /// Register writes update hardware latches. Trigger reloads the oscillator,
    /// envelope and sweep independently of the host's current output chunk.
    pub fn write(&mut self, register: usize, value: u16, first: bool) {
        match register {
            0 => {
                if self.swept_down && value & 8 == 0 {
                    self.active = false;
                }
                self.sweep = value & 0x7f;
            }
            1 => {
                self.envelope = value;
                self.length = 64 - (value & 63) as u8;
                if value & 0xf800 == 0 {
                    self.active = false;
                }
            }
            _ => {
                self.control = value & 0x47ff;
                self.frequency = value & 0x7ff;
                self.phase %= 16 * (2048 - u64::from(self.frequency));
                if value & 0x8000 != 0 {
                    self.active = self.envelope & 0xf800 != 0;
                    if self.length == 0 {
                        self.length = 64;
                    }
                    self.phase = 0;
                    self.step = 0;
                    self.volume = (self.envelope >> 12) as u8;
                    self.envelope_ticks = self.envelope_period();
                    self.sweep_shadow = self.frequency;
                    self.sweep_ticks = self.sweep_period();
                    self.sweep_enabled = self.sweep & 0x77 != 0;
                    self.swept_down = false;
                    if first && self.sweep & 7 != 0 {
                        self.sweep_candidate();
                    }
                }
            }
        }
    }

    fn envelope_period(&self) -> u8 {
        let n = (self.envelope >> 8) as u8 & 7;
        if n == 0 { 8 } else { n }
    }
    fn sweep_period(&self) -> u8 {
        let n = (self.sweep >> 4) as u8 & 7;
        if n == 0 { 8 } else { n }
    }

    /// Overflow disables channel 1, including the prospective second calculation
    /// after a successful sweep update. Decreasing sweeps remember their direction.
    fn sweep_candidate(&mut self) -> Option<u16> {
        let delta = self.sweep_shadow >> (self.sweep & 7);
        let next = if self.sweep & 8 != 0 {
            self.swept_down = true;
            self.sweep_shadow - delta
        } else {
            self.sweep_shadow + delta
        };
        if next > 2047 {
            self.active = false;
            None
        } else {
            Some(next)
        }
    }

    /// Sequencer steps 0/2/4/6 clock length, 2/6 sweep and 7 envelope.
    pub fn sequence(&mut self, step: u8, first: bool) {
        if step & 1 == 0 && self.control & 0x4000 != 0 && self.length != 0 {
            self.length -= 1;
            if self.length == 0 {
                self.active = false;
            }
        }
        if first && step & 3 == 2 {
            self.sweep_ticks = self.sweep_ticks.saturating_sub(1);
            if self.sweep_ticks == 0 {
                self.sweep_ticks = self.sweep_period();
                if self.sweep_enabled
                    && self.sweep & 0x70 != 0
                    && let Some(next) = self.sweep_candidate()
                    && self.sweep & 7 != 0
                {
                    self.sweep_shadow = next;
                    self.frequency = next;
                    self.phase %= 16 * (2048 - u64::from(next));
                    self.control = (self.control & 0x4000) | next;
                    self.sweep_candidate();
                }
            }
        }
        if step == 7 {
            self.envelope_ticks = self.envelope_ticks.saturating_sub(1);
            if self.envelope_ticks == 0 {
                self.envelope_ticks = self.envelope_period();
                if self.envelope & 0x700 != 0 {
                    self.volume = if self.envelope & 0x800 != 0 {
                        (self.volume + 1).min(15)
                    } else {
                        self.volume.saturating_sub(1)
                    };
                }
            }
        }
    }

    pub fn next_edge(&self) -> Option<u64> {
        self.active
            .then(|| 16 * (2048 - u64::from(self.frequency)) - self.phase)
    }

    /// Divider phase survives frequency writes and output drains. The scheduler
    /// includes oscillator edges so PWM samples never skip a duty transition.
    pub fn advance(&mut self, elapsed: u64) {
        if self.active {
            let period = 16 * (2048 - u64::from(self.frequency));
            let total = self.phase + elapsed;
            // Scheduled calls reach at most one divider edge. Keep the
            // general quotient path for callers advancing a longer interval.
            let (steps, phase) = if total < period {
                (0, total)
            } else if total - period < period {
                (1, total - period)
            } else {
                (total / period, total % period)
            };
            self.step = (self.step + steps as usize) & 7;
            self.phase = phase;
        }
    }

    pub fn level(&self) -> i32 {
        const DUTY: [u8; 4] = [0x80, 0x81, 0xe1, 0x7e];
        if !self.active {
            return 0;
        }
        if DUTY[((self.envelope >> 6) & 3) as usize] & (1 << self.step) != 0 {
            i32::from(self.volume)
        } else {
            -i32::from(self.volume)
        }
    }
}
