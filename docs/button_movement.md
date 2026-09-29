# move a square with buttons

Implementation is complete for the controlled-startup demo. Application acceptance
remains open until the Linux, Chrome, and Brave checks below are manually recorded.
No supplied BIOS or commercial-game startup is part of this slice.

## Guest and hardware route

The shared app now loads `roms/buttons.gba`. Its original ARM source selects mode
3/BG2, draws a red 16×16 square at (112, 72), waits for the next VBlank, erases the
old square, reads active-low KEYINPUT, and moves one pixel per held direction.
It clamps the top-left corner to x=0..224 and y=0..144. The CPU additions are
immediate LDRH, ADD, SUB, CMP, TST, and unshifted register MOV, alongside the
existing literal loads, halfword stores, and conditional branches.

The core owns logical buttons and ordered, bounded input transitions on absolute
GBA cycles. Live keys are applied at the current instruction boundary. Scripted
keys are delivered as timed bus work crosses their declared cycle. KEYINPUT and
VCOUNT writes are ignored; DISPSTAT preserves only writable control/compare bits.
VCOUNT advances over all 228 lines. VBlank is asserted on lines 160..226 and is
cleared on line 227. Frame publication and status timing are independent of host
redraws.

HBlank status starts at cycle 1008 of each 1232-cycle line, matching the selected
[mGBA timing model](https://github.com/mgba-emu/mgba/blob/master/include/mgba/internal/gba/video.h)
inspected on 2026-09-30. The existing scanline renderer still samples at cycle 960.
This is an explicit scanline approximation; fine raster effects and IRQ edge
delivery belong to later slices. No reference implementation code was copied.

## Pacing and lifecycle

The session converts monotonic host durations to integer cycles at 16,777,216 Hz.
It retains fractional cycles and uses absolute deadlines, carrying instruction
overshoot forward. Each callback runs at most two frames and has an instruction
budget. Excess host delay is discarded and reported as slow emulation.

Pause, focus loss, and hidden/minimized state release buttons, cancel queued input,
and clear the host anchor. Resume establishes a fresh anchor. Manual pause remains
independent of focus. Reset and ROM replacement clear guest time and pacing state.
The app requests continued wakes only while loaded, active, and unpaused.

The **Replay square demo** control reloads the demo, queues the shared input script
from `roms/buttons/input.rs`, and stops at the frame-9 deadline. It suppresses live
keyboard polling during replay and pauses with the reported final position.
Pause, reset, ROM replacement, or focus loss cancels replay. The runner checks
the app's shared script against the independently frozen manifest events.

## Recorded automated checks

- Core tests: 4 passed, including read-only keypad/status and display phase edges.
- Session tests: 7 passed, including guest movement, identical one-second guest
  positions/timelines/framebuffers at 60 and 144 Hz, pause/focus/overload recovery,
  and an exact replay deadline when a host callback crosses the final checkpoint.
- Red runs reproduced incorrect KEYINPUT reset state, static display status, input
  not reaching the guest, and HBlank status starting too early. Pacing/lifecycle
  and replay-deadline tests were written before their APIs existed and initially
  failed compilation.
- GNU Arm binutils rebuilt both ROMs with their frozen SHA-256 identities.
- Release headless runner passed the previous pixels fixture and guest-store
  mutation check, plus all five moving-square checkpoints. Each movement checkpoint
  checks the mailbox, frame generation, and every one of the 38,400 pixels.
- Native and `wasm32-unknown-unknown` app compilation passed.
- Clippy passed for the four affected crates with warnings denied.

`fixtures/manifest.toml` is now schema version 2: each fixture has an explicit
`verification` table for terminal pixels or bounded movement checkpoints. The
pixels ROM's hash and expected output are unchanged. CI rebuilds both artifacts.

The square ROM is 324 bytes; SHA-256:
`eae58be9de7214f5bfe808970778d3f8ccca5437f97d37ce11986f2f2fe56b6d`.
The guest mailbox at `0x03000000` holds identifier `0x005b`, x, y, and movement-frame
count as consecutive halfwords. The fixture's final checkpoint is frame 9 at
(114, 73). Scanout contains the previous frame's drawing because the guest moves
after VBlank publishes the completed image.

| Frame | Guest x,y | Completed image x,y |
| --- | --- | --- |
| 1 | 113,72 | 112,72 |
| 3 | 115,72 | 114,72 |
| 5 | 115,74 | 115,73 |
| 8 | 114,73 | 114,74 |
| 9 | 114,73 | 114,73 |

## Manual application acceptance still required

Run from `gba-rs`:

```bash
cargo run --locked -p gba-app --release
```

For Chrome and Brave, run:

```bash
trunk --config web/Trunk.toml serve --release
```

Record the following separately on Linux, Chrome, and Brave:

1. The red square starts at (112, 72); arrows move it without leaving trails or
   crossing the screen edges. Released keys stop movement.
2. Pausing for several seconds and resuming produces no jump. A paused app does
   not maintain an idle repaint loop. Reset returns to the starting position.
3. Hold a direction, lose focus, release it outside the app, then return. No key
   remains held. Hiding/restoring a browser tab produces no catch-up jump.
4. Check movement at different available refresh rates: approximately 59.7275
   pixels per second while a direction is held away from an edge.
5. Click **Replay square demo**. The app must pause after nine emulated frames,
   report (114, 73), and show the same square as the headless capture. Repeat at
   available refresh rates. Native/browser live replay has not been run.
6. Record core, conversion, and texture-submission timings from the existing
   Performance panel. No new native/WASM performance measurements are claimed.

The recorded headless capture can be regenerated with:

```bash
cargo run --locked -p gba-tools --release -- fixtures run --capture /tmp/gba-slice2.ppm
```

The capture contains the last fixture's final completed image, at (114, 73).
No computer-use or live application validation was performed for this slice.
