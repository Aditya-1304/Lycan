# Keypad IRQ and HALT verification

KEYCNT selects logical keys, enables keypad IRQ and chooses OR/AND mode.
Requests latch IF bit 12 through the existing IE/IF/IME and firmware exception
path. HALT now stops at queued input deadlines as well as display events;
there is no host wake shortcut. Reserved KEYCNT bits read back as zero.

The original one-shot guests initialize IRQ/system stacks and the callback at
`0x03007FFC`. The callback disables further keypad requests, records IF, performs
normal write-one-to-clear acknowledgement and returns to HALT. Source and frozen
ROM/firmware identities are in `fixtures/manifest.toml`. Both ROMs are 780 bytes.

| Fixture | SHA-256 | First request cycle | Final cycles / instructions | Callback / wake count | IF before / after |
| --- | --- | --- | --- | --- | --- |
| keypad-or | 78530967862d77dd3cfd234cd2f3bcf8726fb38f7d045f67013a2b07c1d81102 | 20,000 | 561,792 / 56 | 1 / 1 | 0x1000 / 0 |
| keypad-and | aec5d725291224e1ec0889d108b3c824586ab6518a88768b2f20cae7001a8d05 | 30,000 | 561,792 / 56 | 1 / 1 | 0x1000 / 0 |

### Plan acceptance

| Requirement | Evidence / status |
| --- | --- |
| KEYCNT selection, enable, OR/AND over active-low input | Both bounded original guests passed |
| Shared IE/IF/IME path | Guest callback reached through mapped firmware; IF source 0x1000, acknowledged to zero; final CPU halted |
| OR/AND fixtures and negative combinations | Right at cycle 10,000 cannot wake either guest; A at 20,000 cannot wake AND; A+B at 30,000 wakes AND. Zero IF/callbacks checked one cycle before the expected request |
| Disabled keypad IRQ | Matching scripted input with KEYCNT enable cleared leaves IF, callbacks and wakes zero; CPU stays halted |
| Same scripted input on all platforms | Native and compiled-WASM contracts passed; Linux/Chrome/Brave replay buttons use the same `roms/keypad/input.rs`; manual runtime acceptance pending |
| Earlier VBlank IRQ/HALT scene unchanged | VBlank eight-frame callback, masked wake, disabled stall, Thumb return and unmapped-vector checks passed; IRQ sprite captures remain identical |

RED: before the core implementation, the bounded OR runner failed because the
expected keypad request did not appear at cycle 20,000. GREEN: OR and AND now
reach their declared request deadlines and complete through the firmware handler.
These fixture checks catch missing or late wake, wrong source, premature IRQ,
failed acknowledgement and disabled-source wake. No new Rust test files were added.

Validation rerun on 2026-09-30:

| Check | Result |
| --- | --- |
| Frozen fixture rebuild | All original ROM rebuilds / upstream identities matched; both keypad ROMs rebuilt with updated hashes |
| Native manifest runner | All 19 fixtures passed |
| Complete workspace suite | 15 tests passed: 8 core, 7 session; app/tools and doc-test targets completed with no tests |
| VBlank regression | Native guest test, five WASM configurations and unmapped-vector rejection passed |
| IRQ sprite regression | All five 38,400-pixel checkpoints passed; 5,617,920 cycles, 59,833 instructions |
| Compiled-WASM runner | 24 WAITCNT cases, ARM/Thumb/services/memory, VBlank variants and both keypad fixtures passed |
| Keypad output | Each fixture: 561,792 cycles, 56 instructions, two completed frames; all 38,400 second-frame pixels equal 0x001F |
| Missing palette-store mutation | Temporary copies of both ROMs with the callback palette store replaced by NOP were rejected at the frame-two comparison after IRQ/mailbox checks |
| Clippy | Workspace all targets, warnings denied: passed |
| Release application builds | Native Linux, wasm32-unknown-unknown and Trunk release: passed |
| Formatting / whitespace | Cargo formatting and git diff whitespace checks passed |

Mutation copies were confined to temporary files; frozen ROMs were preserved.
Existing performance timings were left unchanged. Correctness evidence, hashes,
manual steps and acceptance tables live here, not in `performance.md`.

| Platform | Runtime acceptance |
| --- | --- |
| Linux | Manual visual check pending |
| Chrome | User visual check pending |
| Brave | User visual check pending |

Compiled-WASM execution in Node verifies the production core without a native
fallback. It does not establish browser UI acceptance. No wall-time benchmark
was collected for these one-shot guests; cycle/instruction counts are correctness
budgets, not performance measurements.

### Rendering regression

The realistic bug is a missing or incorrect guest palette store: the IRQ mailbox
can still report success while the screen stays black or almost black. The old
native fixture checks did not compare any pixels. The strengthened native and
compiled-WASM checks first failed on the old guest at frame two (`1` rather than
`0x001F`) and passed after the guest assembly wrote full-intensity red. Both
runners verify two completed-frame generations and compare all 38,400 pixels of
the second frame. The first frame begins before the IRQ, so it is not expected
to be uniformly red. Neither runner nor egui paints a fabricated result.

The WASM harness queues `roms/keypad/input.rs`, shared with the native runner and
app, through `Machine::set_button_at`. Node verifies the exact request deadline,
negative combinations, mailbox identity, callback/wake counts, saved/current IF,
HALT, complete framebuffer, disabled-source stall and unmapped-vector rejection.
The native runner retains the same negative and completion checks and verifies
both completed frames under the declared total instruction/cycle bounds.

The app handles both keypad ROM names before its generic square status branch.
Success requires completion ID `0x0064`, one callback, one wake, saved IF
`0x1000` and saved acknowledgement `0`. A mismatched mailbox reports keypad
failure; it cannot display a square position.

### Reproduce automated verification

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-tools --release -- build-fixtures
cargo run --locked -p gba-tools --release -- fixtures run
python3 roms/cartridge/verify_wasm.py --diagnostics --vblank --keypad
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked -p gba-app --release
cargo build --locked -p gba-app --release --target wasm32-unknown-unknown
env -u NO_COLOR trunk --config web/Trunk.toml build --release
```

### Manual replay

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
env -u NO_COLOR trunk --config web/Trunk.toml serve --release
```

Use the native command for Linux, or open the Trunk URL in Chrome and Brave.
Click **Replay keypad-or.gba**, then **Replay keypad-and.gba**. Each finishes at
561,792 GBA cycles with 56 instructions, one callback, one wake, IF before
`0x1000` and IF after `0`. The second completed framebuffer is uniformly bright red (`0x001F`).
Both replays release all keys and use the exact headless timeline; rerunning
must reproduce the result. Recheck **Replay IRQ sprite input** and the VBlank
demo, plus pause/reset/focus-loss responsiveness. Record platform acceptance and
millisecond timings manually. Linux visual acceptance also remains unverified.
