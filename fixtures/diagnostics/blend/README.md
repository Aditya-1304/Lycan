# Slice 24: alpha blending, brightness and target selection

This original MIT ARM diagnostic uses controlled startup, no BIOS and no
cartridge backup. The guest exposes eight deterministic states:

| State | Behavior |
| ---: | --- |
| 0 | Baseline layer composition |
| 1 | BG0/BG1 alpha blending |
| 2 | Brighten BG0 |
| 3 | Darken BG0 |
| 4 | Semi-transparent OBJ alpha overrides requested OBJ darkening when a valid second target is visible |
| 5 | WIN0 disables ordinary color effects while alpha remains enabled outside WIN0 |
| 6 | WIN0 hides BG1 beneath a semi-transparent OBJ, testing second-target fallback |
| 7 | WIN0 disables ordinary effects while a valid semi-transparent OBJ alpha blend remains active |

OBJ0 crosses WIN0's left edge and uses alternating opaque and transparent
texels. The guest contract checks all 240 by 160 pixels in each state against
an independent color and geometry oracle.

Build and verify the frozen guest with:

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 fixtures/diagnostics/blend/build.py
python3 fixtures/diagnostics/blend/verify.py
cargo run --locked -p gba-tools --release -- bench-blend
```

The fixture identity is frozen in `manifest.toml`; the build script reports
the current size and SHA-256 but never updates the manifest. Native captures
are written to `target/blend-captures/frame-{12,16,20,24,28,32,36,40}.ppm`.

For the manual Linux application replay, run:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release -- --debug-ui
```

For Chrome or Brave, run this in a terminal and open the URL Trunk prints:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release
```

Load `fixtures/diagnostics/blend.gba`. Right advances one state and Left moves to the previous
state; allow each state to settle before pressing again. The guest uses
controlled startup only when the loaded bytes exactly match `fixtures/diagnostics/blend.gba`.

The pinned mGBA suite revision has no standalone blend ROM. The applicable
hardware-verified semi-transparent OBJ case is described in [mGBA issue #3804](https://github.com/mgba-emu/mgba/issues/3804): when a valid second target
overlaps the OBJ, BLDALPHA takes precedence over BLDCNT brighten/darken. State
4 reproduces that behavior without copying the issue attachment, whose reuse
terms are unclear. Blend rounding, green precision and forced-alpha ordering
also follow the independently pinned NanoBoyAdvance reference revision in
`docs/references.md`.
