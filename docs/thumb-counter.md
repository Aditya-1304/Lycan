# ARM and Thumb counter

The original MIT guest in `roms/counter/counter.s` starts in ARM, waits for
VBlank, enters Thumb with BX, samples active-low KEYINPUT, and increments a
counter once per frame while A is held. The count saturates at 16. Thumb BL
calls the increment routine; BX LR returns within Thumb, then BX R9 restores
the even ARM return address. ARM draws eight green pixels per count in a
128x16 bar at (56,72). Reset clears the counter.

## Plan acceptance

- BX changes CPSR.T and refills at the selected 16-bit or 32-bit instruction
  width. Other Thumb PC writes retain state.
- Thumb immediate shifts and ALU operations reuse ARM arithmetic/flag rules.
  High-register ADD, CMP, MOV, and BX support the bridge. Word and halfword
  transfers use the existing timed bus.
- Thumb operands observe instruction address + 4. ADR aligns that value to
  four bytes; the guest deliberately executes ADR at address 2 modulo 4.
- Conditional branches and the two-halfword ARMv4T BL preserve the signed
  target and odd Thumb link address. The final even return restores ARM.
- The shared application exposes **Load counter demo** on native and WASM.
  Hold Z (A) to fill the bar, release to stop, and use Reset to clear it.

## Automated evidence

One focused test was added to the existing core test module. Its realistic
failure is an incorrect interworking pipeline, PC-relative address, or BL
return state; previous fixtures execute exclusively in ARM. The RED run
failed on unsupported BX at `0x0800009c`; GREEN verifies the ARM return at
`0x08000040`, next instruction `0x08000044`, count 1, ADR `0x080000d0`,
observed Thumb PC `0x080000b4`, and link address `0x080000bf`.

The manifest freezes the 220-byte ROM's SHA-256, original source, controlled
ARM startup, lack of BIOS requirement, input events, completion identifier
`0x005f`, exact ARM return address, PC observations, and total cycle and
instruction limits. Every frame must reach the declared return in ARM state;
an arbitrary loop cannot pass. Six checkpoints independently check RAM and
all 38,400 completed-frame pixels:

| Frame | Counter | Displayed counter |
| --- | --- | --- |
| 1 | 1 | 0 |
| 3 | 3 | 2 |
| 5 | 3 | 3 |
| 7 | 5 | 4 |
| 8 | 5 | 5 |
| 24 | 16 | 16 |

VBlank publishes the previous drawing before the guest updates the counter.
The complete run used 6,741,508 cycles and 668,215 instructions, below the
7,000,000-cycle and 1,500,000-instruction bounds.

Fixture rebuild and all earlier fixtures passed. All 12 core/session tests,
workspace Clippy with warnings denied, native and WASM app compilation, and
the Trunk release artifact build passed. The first Trunk invocation failed
writing its staged JavaScript loader; an isolated output build and then a
retry with the ordinary output directory both passed without code changes.

## User-performed platform acceptance

Linux, Chrome, and Brave visual interaction and sustained runtime performance
remain pending. Build success and headless execution do not establish these.
Run each command from `gba-rs`:

```sh
cargo run --locked -p gba-app
```

```sh
env -u NO_COLOR trunk --config web/Trunk.toml serve --release
```

Select **Load counter demo**. Hold Z until all 16 cells fill, release and
verify the value stops, then Reset and verify the bar clears. Repeat short
holds separated by releases, and verify pause/resume and focus loss do not
leave A held. Perform the browser checks in both Chrome and Brave.

This implements the counter bridge only. Full diagnostic instruction-family
completion belongs to the next plan item. The existing bus still uses
default cartridge waits; the separate WAITCNT/prefetch prerequisite has not
been implemented or accepted by this change.
