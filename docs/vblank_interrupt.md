# VBlank interrupt and HALT scene

Slice 10 implements the three requirements in [plan_final.md](../../plan_final.md), section 8, Slice 10.
The IRQ sprite variant shares scene assets and input/update code with the earlier
polling fixture. A build-time assembly conditional changes only interrupt setup,
frame waiting and the callback; rebuilding the polling ROM retains its frozen hash.

## Plan acceptance

| Plan requirement | Implementation and evidence | Status |
| --- | --- | --- |
| Original firmware IRQ entry at 0x18 and return | Vector branches to guest firmware, preserves r0-r3/r12/LR, calls the IWRAM pointer, restores registers and returns with SUBS PC, LR, #4 | Automated checks passed |
| Initialize IRQ stack and callback at 0x03007FFC | Each original IRQ guest initializes banked IRQ SP to 0x03007FA0 and installs its own callback | Automated guest execution passed |
| IE/IF/IME, W1C, exception entry/return, HALTCNT and wake | VBlank source latches IF at line 160. IE selects pending sources; IME and CPSR.I gate exception delivery. HALT wakes from IE & IF regardless of delivery masks | Native and WASM diagnostic checks passed |
| Scene updates through IRQ and sleeps between updates | IRQ sprite replay matches all five earlier scroll/player/overlap captures; callback count equals scene updates and CPU is halted at each checkpoint | Native headless passed; app visual checks pending |
| Disabling source reports expected stall | Configuration 6 disables DISPSTAT VBlank generation; bounded native/WASM runs report zero callbacks and wakes while display time continues | Passed |
| Callback requires mapped exception entry | Native check verifies an unmapped fetch at 0x18; WASM probe rejects advancement and mailbox remains zero | Passed |

A single new Rust regression exercises the original diagnostic ROM. Its first
RED run failed at the guest HALTCNT byte store because that register was unmapped.
GREEN proves the guest sleeps, dispatches through firmware, acknowledges IF,
returns to ARM/Thumb and wakes correctly with delivery masks set. Earlier CPU
and pacing tests did not catch missing hardware sleep or IRQ dispatch. Fixture
verification additionally checks actual scanout and disabled-source behavior.

The hardware timing remains absolute. HALT advances display time without retiring
instructions and `advance_to` clips sleeping work to its caller's deadline.
A sleeping resume PC cannot satisfy `run_until_pc` completion without execution.
Only VBlank generation is required here; keypad IRQ, DMA and STOP behavior remain
outside this slice. This original firmware is not a replacement for a supplied BIOS.

## Reproduce automated checks

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-tools -- build-fixtures
cargo run --locked -p gba-tools --release -- fixtures run
python3 roms/cartridge/verify_wasm.py --diagnostics --vblank
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked -p gba-app --target wasm32-unknown-unknown
cargo run --locked -p gba-tools --release -- bench --scenario irq-sprites --frames 600
```

The default application scene is `irq-sprites.gba`. The load and replay buttons
for the IRQ scene use the same loader, session and logical input route as the
polling sprite scene. Opening an identical original IRQ ROM also selects its
explicit test firmware. Reset preserves that firmware selection.

## Manual app acceptance

Native Linux launch:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

Browser serving, for manual Chrome and Brave checks:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release
```

In each app, check arrow movement, Z priority, X object layout, pause/resume,
reset and the IRQ input replay. At replay completion the player must be (116,74)
and scroll (2,0), matching the earlier sprite scene. Check the VBlank IRQ backdrop
demo button as well: the red intensity cycles while the guest sleeps between
callbacks. Return to the IRQ sprite scene before recording its app timings.
Record each platform's core/conversion/submission means, p95 and sample counts
in the Slice 10 table in `performance.md`. Native visual acceptance and both
browser app checks remain pending; no GUI or browser was launched for validation.

The Trunk release build and WASM app check passed. The Node runtime verifies the
production WASM core's IRQ diagnostic, not browser scheduling or visual playback.
These checks therefore do not complete the pending manual acceptance items.
