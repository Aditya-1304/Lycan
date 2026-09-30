# ARM and Thumb diagnostic evidence

This implements only `plan_final.md` item 7. Tiled backgrounds, sprites, IRQ
callbacks, HALT, and later slices are outside this change.

## Plan acceptance check

| Requirement | Implementation and evidence | Status |
|---|---|---|
| Run `arm.gba` and `thumb.gba` and display results | Pinned upstream binaries, built-in app selectors, and independently derived complete success-screen framebuffer | Headless passed; Linux/Chrome/Brave visual checks pending |
| Finish missing instruction families through reproduced failures | Shared ARM/Thumb ALU, register shifts, long multiplies, transfer/swap forms, stack and block transfers | Both complete diagnostic programs passed |
| CPU status, banked registers, exception and return behavior | MRS/MSR, FIQ/user and exception banks, SPSR, guest SWI entry and exception return | Upstream banking cases and original services guest passed |
| Original firmware with SWI 0x06 | `roms/test-firmware/division.s` mapped at the low firmware window; division executes as guest code | ARM signed division, Thumb division and status restoration passed |
| Hash-bound completion contracts | Manifest binds ROM and firmware SHA-256, exact terminal instruction address/opcode, full CPSR, result register, and instruction/cycle limits | Passed |
| Declared successful completion, never arbitrary self-branch | Exact successful terminal plus zero failure result and reference framebuffer | Passed; injected case-1 failures rejected |

## Fixture identity and bounded completion

Upstream revision: `a7113b67e63f83a9b321696ddd7042ccfad6c881` of
`https://github.com/jsmolka/gba-tests`. The ROMs and their included source files
are retained under `roms/gba-tests`, with the existing upstream MIT notice.
The published FASMARM binaries are verified directly; GNU binutils rebuilds
only the original guests and original test firmware.

| Fixture | Terminal instruction | Result | Full CPSR | Observed completion cycles | Instructions |
|---|---|---|---|---:|---:|
| ARM | `0x08001ec4`: `0xeafffffe` | `r12 = 0` | `0x6000001f` | 244803 | 22887 |
| Thumb | `0x08000aac`: `0xeafffffe` (returns to ARM) | `r7 = 0` | `0x600000df` | 244771 | 22944 |
| Original services guest | `0x08000400`: `0xeafffffe` | `r12 = 0` | `0x600000df` | 1513 | 529 |

Every diagnostic has a 400,000-instruction and 3,000,000-cycle bound.
Scanout runs within the remaining instruction budget and cycle limit.
Completion cycles are emulated time, not a physical-hardware timing claim.

- ARM SHA-256: `77ee88662552bdc885c1080c0172ff119d54db791bd73b21808cf1ff1fe5b40e`
- Thumb SHA-256: `b5cb2291df4ab314b31c598acd9bff2ccfa0b38efff29daadfe97422ce369b67`
- Original services SHA-256: `b12c06f84e1ac37c32d69e4df4205bc7306185b680a1335b20661e364d831453`
- Original firmware SHA-256: `c7e0ef3c08cded916bc6057f5a2214741508db83cb089ce8993e3c7926fc7732`
- Success framebuffer SHA-256 for both upstream programs:
  `59ce42abae9825c2d2579c5cd838e47d88be917e37ea36ff162d46fc5d0991e3`

The framebuffer reference is independent of the emulator: white (`0x7fff`)
240×160 background; `All tests passed` at `(56,76)`; 8×8 glyphs decoded
least-significant-bit first from the pinned `lib/glyphs.asm`; set bits are
black (`0x0000`). Hash the resulting row-major little-endian halfwords.
This follows `lib/macros.inc` and `lib/text.asm`, rather than accepting an
image produced by the implementation as its own expected output.

## Focused RED/GREEN evidence

The realistic missing behavior is CPU execution of instruction families absent
from the earlier counter guest. The existing counter test cannot catch it.
The pinned diagnostics themselves provide guest assertions; no duplicate
unit-test suite was added.

The initial ARM run stopped at unsupported `0xe328f101` at `0x080000f8`
(MSR). Subsequent reproductions stopped at MOV PC, long multiply, shifted
word transfer, signed halfword transfer, SWP, and user-bank block transfer.
The legacy CMP-with-PC case reported failure 234 before SPSR restoration was
implemented. Thumb reproduced missing ADD/SUB, ALU, SP adjustment,
register-offset memory, SP-relative memory, and PUSH/POP forms.
After their shared CPU paths were extended, both guests completed successfully.

The original services guest catches a distinct bug: successful upstream runs
skip the division calls in their failure-reporting path, so they cannot prove
SWI entry, firmware execution, signed results, or ARM/Thumb exception return.
It also reproduced a firmware mapping that omitted the CPU's lookahead fetch
at the return instruction; mapping the complete 16 KiB window fixed that error.

Two temporary mutated guests reproduced real failure reports, without changing
tracked upstream ROMs or expected success results:

- ARM: clear Z in the first MSR (`0x080000f8`) so condition case 1 fails.
- Thumb: change the first zero-producing MOV (`0x080000fe`) to MOV 1 so
  zero-flag case 1 fails.

Both were rejected with `failed case=1` at their declared terminal addresses.
Their complete captured framebuffers independently matched `Failed test 001`
at `(60,76)`, including digits computed by guest SWI division. Reaching the
terminal loop therefore does not turn a failed diagnostic into a pass.

## Automated verification

- Rebuild original guests/firmware and verify frozen hashes: passed.
- Full release fixture runner, including all earlier fixtures: passed.
- Existing core/session tests: 14 passed; no new unit tests.
- Workspace Clippy, all targets, warnings denied: passed.
- Native application check and WASM application check: passed.
- Trunk release browser build: passed.
- Native and browser visual interaction: pending user verification.

## Reproduction and result collection

Run from the worktree. These commands rebuild and verify the original firmware
and guests, retain headless output, and generate the two success-screen captures.

```bash
cd /home/aditya/Projects/GBA/gba-rs
mkdir -p /tmp/gba-diagnostic-results
cargo run --locked -p gba-tools -- build-fixtures
cargo run --locked -p gba-tools --release -- fixtures run --capture /tmp/gba-diagnostic-results/frame | tee /tmp/gba-diagnostic-results/headless.txt
cargo test --locked -p gba-core -p gba-session
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked -p gba-app
cargo check --locked -p gba-app --target wasm32-unknown-unknown
env -u NO_COLOR trunk --config web/Trunk.toml build --release
```

Headless captures: `/tmp/gba-diagnostic-results/frame.arm.ppm` and
`/tmp/gba-diagnostic-results/frame.thumb.ppm`. The `PASS arm`, `PASS thumb`,
and `PASS services` lines contain completion cycles, instruction counts,
CPSR, result registers, and `completion_ms` (ROM loading and execution through
the terminal instruction; excludes subsequent scanout and presentation).

Native Linux:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

Click **Load ARM tests**, verify `All tests passed`, and record the Performance
panel after its sample counts reach 120. Click **Load Thumb tests** and repeat.
Selecting a ROM clears the timing samples. Check Reset reruns each diagnostic;
Pause/Resume preserves the display; resizing preserves the 3:2 image. Regression
check the existing copy, calculation, palette, counter and moving-square controls.

For browser measurements, keep this running in another terminal:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open each browser yourself with these commands, perform the same checks, and
record results separately. Include tab-hide/restore and focus-loss behavior.

```bash
google-chrome-stable http://127.0.0.1:8080
brave http://127.0.0.1:8080
```

The Performance panel reports rolling mean/p95 core callback, pixel-conversion,
and texture-submission costs. Measurements after the success screen describe
steady terminal-loop callbacks, not throughput of the whole diagnostic run.
Use headless `completion_ms` for bounded diagnostic execution cost. Keep power,
build profile and browser conditions consistent with `performance.md`.

## Limits

This firmware is explicitly test-only and implements SWI 0x06 only. Unsupported
services and other exception vectors trap. Division by zero also traps; this
slice does not promise the retail BIOS's complete division ABI or timing.
Normal ROM loading clears the firmware mapping. The app enables it only for
byte-identical pinned ARM/Thumb diagnostics; Reset retains the current mapping.
No host-side division, game-specific instruction result patches, IRQ delivery,
or later-slice graphics were added. Unmapped word data reads used by the
upstream offset case expose a bus latch; this is not exhaustive open-bus proof.
Native/WASM compilation and headless frames do not establish browser runtime
acceptance. Record Linux, Chrome and Brave observations in `performance.md`.
