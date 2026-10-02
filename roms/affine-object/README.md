# Affine object diagnostic

The original MIT guest `sprite.s` rotates and enlarges an asymmetric 32x32
object over the existing opaque text-background path. It uses controlled ARM
startup, no BIOS and no backup. Only exact shipped ROM bytes select the app's
controlled startup path; other cartridges retain supplied-BIOS startup.

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 roms/affine-object/build.py
python3 roms/affine-object/verify.py
cargo run --locked -p gba-tools --release -- bench-affine-object --frames 600
```

The manifest freezes ROM hashes, mailbox IDs, instruction limits and PC windows.
The interactive guest does not terminate: each checkpoint requires its polling
PC, exact seven-slot mailbox, completed framebuffer generation and all 38,400
pixels, with at most 32 cycles of instruction-boundary overshoot. The independent
oracle uses source geometry without tile or palette addressing. The 14 checkpoints
at frames 20 through 72 exercise identity, 45/90/180-degree transforms, negative
fraction flooring, 2x enlargement, normal/expanded bounds, 4/8-bit color, 1D/2D
tile layout, matrix 31, transparent source holes, BG priority and wrapped negative
screen positions. `captures.json` freezes the oracle-validated PPM identities;
building does not update these expectations. Native and production WASM run the
same guest contract. Captures are emitted under `target/affine-object-captures/`.
The benchmark measures 600 steady identity frames after initialization; it
checks the final image and mailbox, drains unused PCM, and excludes app upload.

## mGBA comparison

`degenerate.s` is a standalone ARM translation of the applicable
[`degenerateObjTransform` case](https://github.com/mgba-emu/suite/blob/e6942030d25ffe3ba76c72b73a86da073ec857cc/src/video.c#L106)
from mGBA suite revision `e6942030d25ffe3ba76c72b73a86da073ec857cc`.
The palette, 64 source tiles, six 64x64 objects, base tile 0x240, palette bank 1
and all six singular matrices are preserved. The suite's white backdrop is
initialized explicitly. Its menu, text overlay and BIOS/libgba calls are replaced
by bounded direct startup and a completion mailbox. This runs the applicable
object case, not the entire upstream suite or a separately installed mGBA runtime.

The unmodified upstream `gfx/raw/degenerateObjTransform.bmp` is retained as
`mgba-expected.bmp`, with the upstream MIT license in `mgba-LICENSE`. The verifier
checks its pinned SHA-256, decodes its indexed pixels to BGR555, crops the original
256x128 bitmap to 240 pixels wide and extends its white backdrop to LCD height.
It checks that `mgba-expected.bin` is exactly that derivation before executing the
native and WASM comparison. All 38,400 LCD pixels must match. HBlank OAM updates,
windows, blending and mosaic cases belong to subsequent scope.

## Manual native and browser acceptance

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release -- --debug-ui
```

Load `roms/affine-object.gba` with **Load ROM**, then Resume if paused. No BIOS is
needed. Release each key between presses:

| Host key | Effect |
| --- | --- |
| Right arrow | Cycle identity / 45 / 90 / 180-degree rotation |
| Up arrow | Toggle 1x / 2x enlargement |
| Z (GBA A) | Toggle normal / expanded bounds |
| X (GBA B) | Toggle 4-bit / 8-bit source colors; image stays the same |
| S (GBA R) | Toggle 1D / 2D tile layout; image stays the same |
| A (GBA L) | Toggle in front of / behind the green background |
| Left arrow | Toggle centered / clipped top-left position |

For the captured replay, press Right, Up, Z, X, S, A, A, Left, Right, Right, S,
X, Z in that order, releasing between presses. Compare against frames
24/28/32/36/40/44/48/52/56/60/64/68/72. Reset shows frame 20's initial state.
Check that expanded bounds reveal the enlarged rotated source, transparent holes
show green, priority hides/restores the sprite, and pause/resume/reset, focus-loss
release, resizing and crisp pixels remain working. F1 exposes frame timing.

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --dist /home/aditya/Projects/GBA/gba-rs/target/affine-object-web --address 127.0.0.1 --port 8080
```

Open http://127.0.0.1:8080 in Chrome and Brave. Load the same ROM and repeat the
checks. Record versions, sustained speed and core/conversion/texture submission
mean/p95 milliseconds in `docs/performance.md`. Manual acceptance remains pending
until the user confirms it. The native/WASM headless captures do not prove UI runtime.
