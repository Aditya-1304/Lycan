//! Cartridge GPIO and S-3511 serial clock. Time is supplied by the session;
//! neither bus accesses nor CPU reset consult the host clock.

const EPOCH: u64 = 946_684_800;
const DAYS: u64 = 36_525;
const PERIOD: u64 = DAYS * 86_400;

/// Versioned clock settings, separate from raw cartridge backup data. The anchor
/// and base retain elapsed offline time without rewriting storage every second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RtcImage {
    pub host_anchor: u64,
    pub clock_base: u64,
    pub control: u8,
    pub weekday_bias: u8,
    pub revision: u64,
    pub dirty: bool,
}

impl RtcImage {
    /// Encode configuration only; elapsed time is derived from the saved anchor.
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = b"GBARTC01".to_vec();
        bytes.extend_from_slice(&self.host_anchor.to_le_bytes());
        bytes.extend_from_slice(&self.clock_base.to_le_bytes());
        bytes.extend_from_slice(&[self.control, self.weekday_bias]);
        bytes
    }

    /// Validate the complete representation before changing a live clock.
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != 26 || &bytes[..8] != b"GBARTC01" {
            return Err("Invalid RTC metadata version or length");
        }
        let clock_base = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        if clock_base >= PERIOD || bytes[24] & !0x6a != 0 || bytes[25] >= 7 {
            return Err("Invalid RTC metadata settings");
        }
        Ok(Self {
            host_anchor: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            clock_base,
            control: bytes[24],
            weekday_bias: bytes[25],
            revision: 0,
            dirty: false,
        })
    }
}

pub(crate) struct Rtc {
    pub image: RtcImage,
    now: u64,
    pub initialized: bool,
    data: u16,
    direction: u16,
    enabled: bool,
    command: u8,
    bits: usize,
    buffer: [u8; 7],
    output: u16,
}

impl Rtc {
    pub fn new() -> Self {
        Self {
            image: RtcImage {
                host_anchor: 0,
                clock_base: 0,
                control: 0x40,
                weekday_bias: 0,
                revision: 1,
                dirty: true,
            },
            now: 0,
            initialized: false,
            data: 0,
            direction: 0,
            enabled: false,
            command: 0,
            bits: 0,
            buffer: [0; 7],
            output: 2,
        }
    }

    /// Initialize an unconfigured clock once. Later samples advance the anchor
    /// algebraically, including pause/offline intervals and host clock corrections.
    pub fn set_time(&mut self, now: u64) {
        if !self.initialized {
            self.initialized = true;
            self.image.host_anchor = now;
            self.image.clock_base = now.saturating_sub(EPOCH) % PERIOD;
        }
        self.now = now;
    }

    fn seconds(&self) -> u64 {
        (i128::from(self.image.clock_base) + i128::from(self.now)
            - i128::from(self.image.host_anchor))
        .rem_euclid(i128::from(PERIOD)) as u64
    }

    pub fn reset_bus(&mut self) {
        self.data = 0;
        self.direction = 0;
        self.enabled = false;
        self.command = 0;
        self.bits = 0;
        self.output = 2;
    }

    pub fn read(&self, address: u32) -> Option<u16> {
        if !self.enabled {
            return None;
        }
        match address {
            0x080000c4 => Some((self.data & self.direction) | (self.output & !self.direction)),
            0x080000c6 => Some(self.direction),
            0x080000c8 => Some(1),
            _ => None,
        }
    }

    /// CS begins/aborts a transaction. Rising SCK samples host bits; falling
    /// SCK presents the next read bit, retained until the following falling edge.
    pub fn write(&mut self, address: u32, value: u16) -> bool {
        match address {
            0x080000c6 => {
                self.direction = value & 15;
                return true;
            }
            0x080000c8 => {
                self.enabled = value & 1 != 0;
                return true;
            }
            0x080000c4 => {}
            _ => return false,
        }
        let old = self.data;
        self.data = value & self.direction & 15;
        if self.data & 4 == 0 || old & 4 == 0 {
            self.bits = 0;
            self.command = 0;
            self.buffer = [0; 7];
            self.output = 2;
            return true;
        }
        if self.command & 1 != 0 && self.command != 0 {
            if old & 1 != 0 && self.data & 1 == 0 {
                let length = self.length();
                if length > 0 {
                    let bit = self.bits % (length * 8);
                    self.output = u16::from((self.buffer[bit / 8] >> (bit % 8)) & 1) << 1;
                    self.bits += 1;
                }
            }
        } else if old & 1 == 0 && self.data & 1 != 0 {
            if self.bits < 56 {
                self.buffer[self.bits / 8] |= ((self.data >> 1) as u8 & 1) << (self.bits % 8);
                self.bits += 1;
            }
            if self.command == 0 && self.bits == 8 {
                let mut command = self.buffer[0];
                if command & 0xf0 != 0x60 {
                    command = command.reverse_bits();
                }
                self.bits = 0;
                self.buffer = [0; 7];
                if command & 0xf0 == 0x60 {
                    self.command = command;
                    if command & 0x0e == 0 {
                        self.image.control = 0;
                        self.changed();
                    }
                    self.latch();
                }
            } else if self.command != 0 && self.length() > 0 && self.bits == self.length() * 8 {
                self.commit();
                self.bits = 0;
                self.buffer = [0; 7];
            }
        }
        true
    }

    fn length(&self) -> usize {
        match self.command & 0x0e {
            2 => 7,
            4 => 1,
            6 => 3,
            _ => 0,
        }
    }

    fn changed(&mut self) {
        self.initialized = true;
        self.image.revision = self.image.revision.wrapping_add(1);
        self.image.dirty = true;
    }

    fn latch(&mut self) {
        let seconds = self.seconds();
        let days = seconds / 86_400;
        let (year, month, day) = date(days);
        let hour = (seconds / 3600 % 24) as u8;
        let hour = if self.image.control & 0x40 != 0 {
            bcd(hour)
        } else {
            bcd(hour % 12) | if hour >= 12 { 0x80 } else { 0 }
        };
        let values = [
            bcd(year),
            bcd(month),
            bcd(day),
            ((days + 6 + u64::from(self.image.weekday_bias)) % 7) as u8,
            hour,
            bcd((seconds / 60 % 60) as u8),
            bcd((seconds % 60) as u8),
        ];
        match self.command & 0x0e {
            2 => self.buffer = values,
            4 => self.buffer[0] = self.image.control,
            6 => self.buffer[..3].copy_from_slice(&values[4..]),
            _ => self.buffer.fill(0xff),
        }
        if self.command & 1 == 0 {
            self.buffer.fill(0);
        }
    }

    /// Accept only complete valid BCD settings; an interrupted transaction never
    /// changes persisted state. Time-only writes retain the current calendar day.
    fn commit(&mut self) {
        if self.command & 0x0e == 4 {
            self.image.control = self.buffer[0] & 0x6a;
            self.changed();
            return;
        }
        let full = self.command & 0x0e == 2;
        let offset = if full { 4 } else { 0 };
        let raw_hour = self.buffer[offset];
        let Some(mut hour) = unbcd(raw_hour & 0x7f) else {
            return;
        };
        if self.image.control & 0x40 == 0 {
            if hour >= 12 {
                return;
            }
            if raw_hour & 0x80 != 0 {
                hour += 12;
            }
        } else if raw_hour & 0x80 != 0 {
            return;
        }
        let (Some(minute), Some(second)) = (
            unbcd(self.buffer[offset + 1]),
            unbcd(self.buffer[offset + 2]),
        ) else {
            return;
        };
        if hour >= 24 || minute >= 60 || second >= 60 {
            return;
        }
        let mut days = self.seconds() / 86_400;
        if full {
            let (Some(year), Some(month), Some(day)) = (
                unbcd(self.buffer[0]),
                unbcd(self.buffer[1]),
                unbcd(self.buffer[2]),
            ) else {
                return;
            };
            if !(1..=12).contains(&month)
                || day == 0
                || day > month_days(year, month)
                || self.buffer[3] >= 7
            {
                return;
            }
            days = (0..year)
                .map(|y| if y % 4 == 0 { 366u64 } else { 365 })
                .sum::<u64>()
                + (1..month)
                    .map(|m| u64::from(month_days(year, m)))
                    .sum::<u64>()
                + u64::from(day - 1);
            self.image.weekday_bias = ((u64::from(self.buffer[3]) + 7 - (days + 6) % 7) % 7) as u8;
        }
        self.image.clock_base =
            days * 86_400 + u64::from(hour) * 3600 + u64::from(minute) * 60 + u64::from(second);
        self.image.host_anchor = self.now;
        self.changed();
    }
}

fn bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}
fn unbcd(value: u8) -> Option<u8> {
    (value >> 4 < 10 && value & 15 < 10).then_some((value >> 4) * 10 + (value & 15))
}
fn month_days(year: u8, month: u8) -> u8 {
    match month {
        2 => {
            if year.is_multiple_of(4) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}
fn date(mut days: u64) -> (u8, u8, u8) {
    let mut year: u8 = 0;
    loop {
        let length = if year.is_multiple_of(4) { 366 } else { 365 };
        if days < length {
            break;
        }
        days -= length;
        year += 1;
    }
    let mut month = 1;
    while days >= u64::from(month_days(year, month)) {
        days -= u64::from(month_days(year, month));
        month += 1;
    }
    (year, month, days as u8 + 1)
}
