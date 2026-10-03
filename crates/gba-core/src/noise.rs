//! Channel-four polynomial noise, clocked by the machine and PSG sequencer.
//! Register latches and synthesis history remain independent of host PCM drains.

/// Seven- or fifteen-stage noise generator with a 256 Hz length counter and
/// 64 Hz volume envelope. The mixer owns routing and master-volume scaling.
#[derive(Default)]
pub(super) struct Noise {
    pub envelope: u16,
    pub control: u16,
    pub active: bool,
    phase: u64,
    polynomial: u16,
    high: bool,
    length: u8,
    volume: u8,
    envelope_ticks: u8,
}

impl Noise {
    /// Updates writable latches; a trigger reloads the polynomial and envelope.
    /// DAC disable stops the voice immediately, without affecting other voices.
    pub fn write(&mut self, offset: usize, value: u16) {
        if offset == 0x78 {
            self.envelope = value & 0xff3f;
            self.length = 64 - (value & 63) as u8;
            if value & 0xf800 == 0 {
                self.active = false;
            }
        } else {
            self.control = value & 0x40ff;
            self.phase %= self.period();
            if value & 0x8000 != 0 {
                self.active = self.envelope & 0xf800 != 0;
                if self.length == 0 {
                    self.length = 64;
                }
                self.phase = 0;
                self.polynomial = if value & 8 != 0 { 0x40 } else { 0x4000 };
                self.high = false;
                self.volume = (self.envelope >> 12) as u8;
                self.envelope_ticks = self.envelope_period();
            }
        }
    }

    /// Converts the hardware divisor and shift into master-clock cycles.
    /// Divisor zero denotes one half; all other divisors use their numeric value.
    fn period(&self) -> u64 {
        let divisor = self.control & 7;
        (if divisor == 0 {
            32
        } else {
            64 * u64::from(divisor)
        }) << ((self.control >> 4) & 15)
    }

    fn envelope_period(&self) -> u8 {
        let period = (self.envelope >> 8) as u8 & 7;
        if period == 0 { 8 } else { period }
    }

    /// Even sequencer steps clock length; step seven clocks the envelope.
    /// A zero envelope period keeps the trigger volume constant.
    pub fn sequence(&mut self, step: u8) {
        if step & 1 == 0 && self.control & 0x4000 != 0 && self.length != 0 {
            self.length -= 1;
            if self.length == 0 {
                self.active = false;
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
        self.active.then(|| self.period() - self.phase)
    }

    /// Advances the polynomial at device edges, including edges between PWM
    /// samples. The scheduler bounds elapsed time to the next oscillator edge.
    pub fn advance(&mut self, elapsed: u64) {
        if self.active {
            let total = self.phase + elapsed;
            let period = self.period();
            // Avoid runtime division at the usual zero/one-edge boundary.
            // A longer explicit advance still clocks every polynomial step.
            let (steps, phase) = if total < period {
                (0, total)
            } else if total - period < period {
                (1, total - period)
            } else {
                (total / period, total % period)
            };
            for _ in 0..steps {
                self.high = self.polynomial & 1 != 0;
                self.polynomial >>= 1;
                if self.high {
                    self.polynomial ^= if self.control & 8 != 0 { 0x60 } else { 0x6000 };
                }
            }
            self.phase = phase;
        }
    }

    pub fn level(&self) -> i32 {
        if !self.active {
            0
        } else if self.high {
            i32::from(self.volume)
        } else {
            -i32::from(self.volume)
        }
    }
}
