# Cartridge clock diagnostic

This original ARM guest uses the production GPIO, RTC, SRAM, session, and app
storage paths. It runs through controlled cartridge startup without a BIOS.
The versioned RTC marker selects clock hardware; ordinary cartridges keep ROM
reads and ignored ROM writes.

The eight green decimal rows show year, month, day, weekday (Sunday = 0), hour,
minute, second, and control. The guest initially configures Thursday,
2024-02-29 23:59:58 in 24-hour mode and stores an initialization byte in SRAM.
Reset and reopening read this byte and leave the clock settings intact.

- **Z / GBA A** configures that date and 24-hour mode again.
- **X / GBA B** selects 12-hour mode. The displayed hour includes the hardware
  PM flag: 23:xx becomes BCD `0x91`, displayed as `91`; midnight displays `00`.
- Release the button after each action. The guest continuously polls the clock.

The application supplies Unix seconds even while paused or inactive. New clocks
use UTC; guest settings select a new clock base. Elapsed time follows the supplied
host clock, including offline intervals and corrections. The core never samples
an operating-system clock. Headless replay records `RtcTimeEvent` samples with
exact guest-cycle boundaries and applies them using `replay_rtc_time`. The
retained `time_events.rs` timeline contains samples at cycles 0 and 1,404,485.

`GBARTC01` encodes a host anchor, configured clock base, clock control, and weekday
bias. It is embedded alongside raw backup bytes in the identity-bearing atomic
`GBACART1` storage record. Storage acknowledges backup and RTC revisions separately;
reset preserves both settings and pending revisions, while clearing GPIO transfer
state. Existing backup-only storage records remain readable. `.sav` exports contain
only raw backup bytes; importing backup bytes leaves clock metadata intact.

The protocol uses CS framing, LSB-first data, normal/reversed command order,
BCD date/time and 12/24-hour control. GPIO/serial edge behavior was checked against
[mGBA's cartridge GPIO implementation](https://github.com/mgba-emu/mgba/blob/master/src/gba/cart/gpio.c).
The diagnostic exercises reads, date writes, control writes and lifecycle retention;
per-minute cartridge IRQ signaling is not part of this clock-read contract.

## Automated verification

Run from `/home/aditya/Projects/GBA/gba-rs`:

```bash
python3 roms/rtc/build.py
cargo run --locked -p gba-tools --release -- verify-rtc --capture target/rtc.ppm
cargo test --locked -p gba-session guest_reads_configured_cartridge_clock
```

The dedicated manifest freezes ROM and final framebuffer identities. Each
checkpoint runs five frames, at most 200,000 instructions per frame and
1,404,512 cycles total. The contract checks the polling/drawing PC window,
mailbox `0x30`, exact date/control bytes and all 38,400 scanout pixels. It checks
leap-day rollover, paused elapsed time, reset, reopen, 12-hour settings,
stale storage acknowledgement, malformed metadata and recorded time replay.
The one session regression failed before RTC support because reads returned ROM
bytes instead of the configured date.

Separate-process persistence uses the app's native storage implementation. Use a
fresh directory for the first command; reuse it for the second:

```bash
RTC_SAVE_DIR="$(mktemp -d /tmp/gba-rtc-check.XXXXXX)"
GBA_SAVE_DIR="$RTC_SAVE_DIR" cargo run --locked -p gba-app --release -- --save-probe rtc-write
GBA_SAVE_DIR="$RTC_SAVE_DIR" cargo run --locked -p gba-app --release -- --save-probe rtc-read
```

The probe injects a fixed 65-second offline interval, validates identity/truncation
rejection and failed replacement handling, and verifies a 32,768-byte raw export.

## Manual browser acceptance

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in Chrome and Brave, separately. Load
`roms/rtc.gba` through the ROM picker. No BIOS is needed for this diagnostic.

1. On first load, verify the configured date above and advancing seconds.
2. Pause for at least ten seconds, then resume. The displayed time must include
   the paused interval. Repeat with the tab hidden.
3. Press/release Z, then X. Control must become `00`; hour must display `91` near
   the freshly configured 23:59:58 value. Verify the leap-day/midnight rollover.
4. Reset. Clock settings and elapsed time must survive.
5. Wait for the app to report saved, close the tab, wait at least fifteen seconds,
   then reopen the same origin/profile and load the same ROM. The date and clock
   mode must survive, with elapsed offline time included. Z intentionally changes
   the configured date, so do not press it during this retention check.
6. Export save. The `.sav` must be exactly 32,768 bytes. Import that export and
   verify clock settings remain intact.

Record Chrome/Brave results against the exact ROM SHA-256 in `manifest.toml`.
Native visual acceptance, if desired, uses `cargo run --locked -p gba-app --release`
and the same picker, buttons and lifecycle sequence. Visual checks remain pending
until the user confirms them; headless verification does not mark them complete.
