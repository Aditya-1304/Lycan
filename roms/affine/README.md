# Affine background image contract

Original MIT ARM guests, assembled from `image.s` with MODE=1..5. These are
continuous interactive diagnostics with controlled cartridge startup, no BIOS,
and no cartridge backup. The app recognizes their exact bytes through its
existing diagnostic startup route. Other cartridges retain supplied-BIOS boot.

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 roms/affine/build.py
python3 roms/affine/verify.py
```

The verifier rejects altered ROM hashes, runs the release native core, saves PPM
captures under `target/affine-captures`, checks their frozen SHA-256 identities,
and runs the same geometry oracle against the production WASM core in Node.
`captures.json` freezes the 35 oracle-validated captures; rebuilding does not
rewrite expected identities. Native capture generation is also available with:

```bash
cargo run --locked -p gba-tools --release -- verify-affine --capture target/affine-captures
```

Each variant uses an independently defined source gradient. Mode 1 exercises
BG2; mode 2 exercises BG3. Modes 3/4 transform 240x160 bitmaps; mode 5 transforms
160x128 direct-color pages at offsets 0 and 0xa000. Mode 3 ignores page selection.
The guest centers the source on the LCD, rotates in quarter turns and scales
using integer reference coordinates and signed 8.8 matrix coefficients.

The checkpoint mailbox at 0x03000000 contains ID 0x00a1, angle, scale, page and
wrap state. Frames 20/24/28/32/36/40/44 each require the declared polling PC window, exact mailbox, completed scanout and all
38,400 pixels, with at most 2,000,000 instructions per advance. The final target
is 12,359,424 cycles, allowing at most 32 cycles of instruction-boundary overshoot. This interactive
guest does not terminate: reaching an arbitrary polling branch is never success.
The oracle calculates geometry from the expected controls independently of the
emulator's affine accumulator and tile/map addressing. Both native and WASM must
match every expected pixel. Scanline rendering remains the existing approximation.

## Manual native and browser checks

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release -- --debug-ui
```

Load each `roms/affine-mode-{1,2,3,4,5}.gba` with **Load ROM**, then Resume if paused.
No BIOS is needed. Arrow Right rotates; Arrow Up toggles source scale between
1x and 2x; Z (GBA A) selects the alternate bitmap page; X (GBA B) toggles tiled
wrapping. Release each key between presses. Mode 3 ignores page selection;
bitmap modes clip regardless of wrapping. Check negative-coordinate clipping,
tiled repetition, both mode-4/5 pages, pause/resume and reset. Reset restores the
initial centered image. F1 shows diagnostics and timing summaries.

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open http://127.0.0.1:8080 in Chrome and Brave and repeat the same file/control
checks. Native and browser manual acceptance is pending until confirmed by the
user. Record browser versions, sustained speed, and core/conversion/texture
submission mean/p95 milliseconds in `docs/performance.md`.

Affine register semantics reference:
[GBATEK](https://mgba-emu.github.io/gbatek/#lcd-io-bg-rotationscaling).
