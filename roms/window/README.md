# Window and mosaic scene

Original MIT ARM diagnostic with controlled startup, no BIOS and no backup.
Only exact shipped bytes select controlled startup in the application.

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 roms/window/build.py
python3 roms/window/verify.py
cargo run --locked -p gba-tools --release -- bench-window
```

The continuous guest updates registers in VBlank. WIN0 overrides WIN1, then
opaque OBJWIN texels override WINOUT. Backdrop remains visible when every layer
is masked. BG0 is red/blue checkerboard, BG1 green, ordinary OBJ white with
transparent holes. WIN0 admits BG0/BG1/OBJ and effects, WIN1 only BG1, OBJWIN
only BG0, outside only BG1/OBJ. A toggles WIN0 to BG1/OBJ, B toggles 4x4 BG
and 3x3 OBJ mosaic. Mosaic samples before source transforms and destination
window selection. Bit 5 of each selected mask retains color-effect permission;
color effects themselves belong to section 24.

The independent geometry oracle compares every LCD pixel at 11 recorded frames,
plus exact four-slot mailbox, polling PC, framebuffer generation and bounded
instruction/cycle progression. Frozen ROM/capture identities are separate from
the builder. Production WASM repeats the same contract in Node. This is headless
execution, with no claim of browser UI acceptance. The guest covers rectangular
boundaries/overlap, object-window transparency, wrapped/empty horizontal bounds,
and both horizontal/vertical text-BG and regular-OBJ mosaic. Affine mosaic follows
the existing sampler but is not independently captured by this guest.

## Manual application checks

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release -- --debug-ui
```

Load `roms/window.gba`, then Resume. Right/Left move WIN0 by 16 pixels, Z
(GBA A) toggles its layers, X (GBA B) toggles mosaic. Up selects wrapped WIN0
bounds (220..255 and 0..19); Down selects an empty WIN0. Reset restores normal
bounds and initial controls. Release keys between presses. Exact capture replay:
Right, Z, X, Right, Right, Left, X, Z, Up, Down. Compare frames 16 through 52
in `target/window-captures/`; initial frame is 12. WIN0 wins overlaps; outside
remains green with visible white OBJ texels. Object-window holes preserve the
outside green layer. Check pause/resume/reset, focus-loss release and crisp resize.
F1 shows core/conversion/texture-submission mean/p95 timing summaries.

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --dist /home/aditya/Projects/GBA/gba-rs/target/window-web --address 127.0.0.1 --port 8080
```

Open http://127.0.0.1:8080 in Chrome and Brave and repeat the same checks. Record
browser versions, sustained speed, and observed timing summaries in performance.md.
