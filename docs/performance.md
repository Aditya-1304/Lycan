# Performance and reference device

This document records the reference environment for repeatable native and browser measurements. Slice 0 verifies presentation and scheduling behavior; it does not yet measure emulation throughput.

## Reference device

- Recorded: 2026-09-29
- Operating system: Arch Linux (rolling)
- Desktop session: Hyprland on Wayland
- CPU: Intel Core i7-13620H, 10 cores / 16 threads
- Display: Chimei Innolux 1920×1080 at 60.001 Hz, scale 1
- Audio output: Built-in Audio Analog Stereo via PipeWire 1.6.8
- Power conditions: AC connected, performance profile

## Toolchain and build settings

- `rustc`: 1.98.1 (48a229cea, 2026-09-01)
- `cargo`: 1.98.1 (797e8a9bc, 2026-08-05)
- `trunk`: 0.21.14
- Native measurement profile: release, `opt-level = 3`, thin LTO
- Browser build: Trunk release build targeting `wasm32-unknown-unknown`
- Browser extensions affecting measurements: none reported
- Tracing: not present in Slice 0

## Browser versions

- Google Chrome: 153.0.8010.47
- Brave: 153.1.95.102

## Slice 0 verification status

- Native release build: passed; the app stayed running during a five-second launch smoke check. UI interaction was not manually observed.
- Browser release build and local Trunk page: passed; Chrome/Brave interaction was not verified because no browser UI was available in the check environment.
- Performance measurements: deferred until runnable emulation and audio exist
- Comparative native/WASM benchmark runs: not performed
- Brave browser build: passed.
- 240×160 animated test image verified on all targets.
- Resize and 3:2 aspect preservation verified.
- Keyboard-to-logical-button mapping verified.
- Focus loss releases held buttons.
- Pause stops animation and does not busy-spin.
- Reset restores initial Slice 0 state.

## Display pixel written by ROM status

### Headless

- Fixture: `pixels.gba`
- SHA-256: `4fb24cb1f126ac04638a3c5819ecee7f3c4fd0bc2b948e186daa79bbdc9c1a12`
- Terminal PC: `0x08000200`
- Completion mailbox: passed
- Exact 240×160 framebuffer verification: passed
- Guest-store mutation test: passed

### Native Linux

- Mode 3 image visually verified:
- Core execution mean: 0.946 ms
- Core execution p95: 1.531 ms
- BGR555 conversion mean: 0.119 ms
- BGR555 conversion p95: 0.168 ms
- Texture submission mean: 0.026 ms
- Texture submission p95: 0.041 ms

### Google Chrome

- Mode 3 image visually verified:
Core execution: mean 1.004 ms | p95 1.400 ms | samples 120
Pixel conversion: mean 0.202 ms | p95 0.300 ms | samples 120
Texture submission: mean 0.021 ms | p95 0.100 ms | samples 120

### Brave

- Mode 3 image visually verified:
Core execution: mean 1.085 ms | p95 1.300 ms | samples 120
Pixel conversion: mean 0.211 ms | p95 0.300 ms | samples 120
Texture submission: mean 0.032 ms | p95 0.100 ms | samples 120

## moving square with button status

Integer master-clock pacing, deterministic 60/144 Hz host-schedule checks, and
bounded pause/focus recovery passed. The moving-square headless fixture verified
all five declared positions and complete framebuffers. Native and WASM compilation
passed. See [button movement evidence and manual acceptance](button_movement.md).

### Headless

- Fixture: `buttons.gba`
- SHA-256: `eae58be9de7214f5bfe808970778d3f8ccca5437f97d37ce11986f2f2fe56b6d`
- Active-low KEYINPUT test: passed
- VCOUNT/DISPSTAT phase tests: passed
- Moving-square checkpoints: passed
- Full framebuffer comparisons: passed
- 60 Hz / 144 Hz deterministic host-schedule test: passed
- Pause/focus re-anchor test: passed
- Replay deadline test: passed
- Final replay position: `(114, 73)`

### Native Linux

- Square movement visually verified: yes
- Boundary clamping verified: yes
- Pause/resume without catch-up: yes
- Focus-loss input release: yes
- Reset: yes
- Replay final position `(114, 73)`: yes
Core execution: mean 1.828 ms | p95 2.369 ms | samples 120
Pixel conversion: mean 0.128 ms | p95 0.168 ms | samples 120
Texture submission: mean 0.026 ms | p95 0.036 ms | samples 120

### Google Chrome

- Square movement visually verified: yes
- Pause/resume without catch-up: yes
- Focus/tab-loss input release: yes
- Hidden-tab restore without catch-up: yes
- Replay final position `(114, 73)`: yes
Core execution: mean 2.359 ms | p95 2.900 ms | samples 120
Pixel conversion: mean 0.230 ms | p95 0.300 ms | samples 120
Texture submission: mean 0.031 ms | p95 0.100 ms | samples 120

### Brave

- Square movement visually verified: yes
- Pause/resume without catch-up: yes
- Focus/tab-loss input release: yes
- Hidden-tab restore without catch-up: yes
- Replay final position `(114, 73)`: yes
Core execution: mean 2.463 ms | p95 3.100 ms | samples 120
Pixel conversion: mean 0.243 ms | p95 0.300 ms | samples 120
Texture submission: mean 0.031 ms | p95 0.100 ms | samples 120


## switch palette colors and bitmap pages status

Mode 4 palette lookup, bitmap page selection, palette-only updates, and the
pinned gba-tests `hello.gba` fixture passed headlessly. The original palette
fixture verifies complete 240×160 framebuffers for both bitmap pages and a
palette-only change without rewriting VRAM.

### Headless

- Fixture: `palette.gba`
- SHA-256: `88806d5738b0d298918b3b59ce67bb08504584f948312614ba8a2977927efc3b`
- Palette RAM access: passed
- Mode 4 indexed lookup: passed
- Page 0 framebuffer: passed
- Page 1 framebuffer: passed
- Palette-only framebuffer change: passed
- Full 38,400-pixel comparisons: passed
- Frame 4: page 0, red/green columns
- Frame 6: page 1, blue/white columns
- Frame 8: page 1 with palette change, blue/red columns

### gba-tests hello

- Fixture: `hello.gba`
- SHA-256: `38aed48b67bc0f701e8aa222b0c3334bd306bd29888707bb7224d81f5576c264`
- Source revision: `a7113b67e63f83a9b321696ddd7042ccfad6c881`
- Terminal PC: `0x08000160`
- Expected framebuffer SHA-256: `56cd131fb3915fe7e410be228a8c09e99132064799f148583636ca75745bedf7`
- Headless framebuffer verification: passed
- `Hello world!` output: passed

### Native Linux

- Palette page 0 visually verified: yes
- Palette page 1 visually verified: yes
- Palette-only update visually verified: yes
- `hello.gba` visually verified: yes
- Pause/reset/focus behavior verified: yes
- Earlier Slice 1/2 scenes regression checked: yes
Core execution: mean 2.142 ms | p95 3.190 ms | samples 120
Pixel conversion: mean 0.122 ms | p95 0.176 ms | samples 120
Texture submission: mean 0.027 ms | p95 0.039 ms | samples 120

### Google Chrome

- Palette page 0 visually verified: yes
- Palette page 1 visually verified: yes
- Palette-only update visu- Core execution mean: TODO ms
- Core execution p95: TODO ms
- Pixel conversion mean: TODO ms
- Pixel conversion p95: TODO ms
- Texture submission mean: TODO ms
- Texture submission p95: TODO msally verified: yes
- `hello.gba` visually verified: yes
- Pause/reset/focus behavior verified: yes
- Earlier Slice 1/2 scenes regression checked: yes
Core execution: mean 2.938 ms | p95 3.800 ms | samples 120
Pixel conversion: mean 0.234 ms | p95 0.300 ms | samples 120
Texture submission: mean 0.032 ms | p95 0.100 ms | samples 120

### Brave

- Palette page 0 visually verified: yes
- Palette page 1 visually verified: yes
- Palette-only update visually verified: yes
- `hello.gba` visually verified: yes
- Pause/reset/focus behavior verified: yes
- Earlier Slice 1/2 scenes regression checked: yes
Core execution: mean 2.262 ms | p95 3.400 ms | samples 120
Pixel conversion: mean 0.188 ms | p95 0.300 ms | samples 120
Texture submission: mean 0.028 ms | p95 0.100 ms | samples 120

## cpu calculation status

The original CPU calculation fixture executes selected ARM arithmetic, logical,
comparison, barrel-shifter, carry, and multiplication cases entirely as guest
code. Each case compares against a fixed expected result and draws its status
through the existing Mode 3 display route.

### Headless

- Fixture: `calculations.gba`
- SHA-256: `8ef128a96363775c3a6b0b3aad2997edfa48e43ba95a920121e9b7a04b4604f5`
- Terminal PC: `0x08001000`
- Completion mailbox ID: `0x005d`
- Completion result: `1`
- First failing case: `0`
- Selected calculation cases passed: `16 / 16`
- Full 240×160 framebuffer verification: passed
- Diagnostic completion host time: TODO ms

### Native Linux

- Diagnostic visually verified: yes
- All calculation bands passed: yes
- Completion mailbox `0x005d`: yes
- Completion result `1`: yes
- First failing case `0`: yes
- Earlier Slice 1–3 scenes regression checked: yes
Core execution: mean 1.863 ms | p95 2.962 ms | samples 120
Pixel conversion: mean 0.110 ms | p95 0.157 ms | samples 120
Texture submission: mean 0.026 ms | p95 0.041 ms | samples 120

### Google Chrome

- Diagnostic visually verified: yes
- All calculation bands passed: yes
- Completion mailbox `0x005d`: yes
- Completion result `1`: yes
- First failing case `0`: yes
- Earlier Slice 1–3 scenes regression checked: yes
Core execution: mean 2.044 ms | p95 6.700 ms | samples 120
Pixel conversion: mean 0.156 ms | p95 0.300 ms | samples 120
Texture submission: mean 0.029 ms | p95 0.100 ms | samples 120

### Brave

- Diagnostic visually verified: yes
- All calculation bands passed: yes
- Completion mailbox `0x005d`: yes
- Completion result `1`: yes
- First failing case `0`: yes
- Earlier Slice 1–3 scenes regression checked: yes
Core execution: mean 2.125 ms | p95 2.800 ms | samples 120
Pixel conversion: mean 0.227 ms | p95 0.300 ms | samples 120
Texture submission: mean 0.028 ms | p95 0.100 ms | samples 120

## copy and redraw an image status

The original bitmap-copy fixture copies a 38,400-byte indexed image through an
EWRAM mirror and redraws it from canonical EWRAM through a called ARM routine.
The routine saves registers and LR on a mirrored IWRAM stack and returns by
loading PC through LDM. The guest verifies stack restoration before publishing
completion.

### Headless

- Fixture: `copy.gba`
- SHA-256: `70ae32d6d49cfbbe8d05f66da4fa22a1076838b0753cab0a9827f304cb695306`
- Terminal PC: `0x0800a000`
- Completion mailbox ID: `0x005e`
- Completion result: `1`
- Full 38,400-byte copied-image verification: passed
- Full 240×160 framebuffer verification: passed
- EWRAM mirror verification: passed
- IWRAM/stack mirror verification: passed
- BL/routine return verification: passed
- SP restoration verification: passed
- Unaligned STR align-down verification: passed
- Unaligned LDR rotation verification: passed
- Palette byte-write duplication: passed
- BG VRAM byte-write duplication: passed
- OBJ VRAM byte-write ignore behavior: passed
- Guest execution cycles: `1687892`
- Copy/redraw completion host time: TODO ms

### Native Linux

- Copied image visually verified: yes
- Earlier Slice 1–4 scenes regression checked: yes
Core execution: mean 2.202 ms | p95 3.093 ms | samples 120
Pixel conversion: mean 0.125 ms | p95 0.157 ms | samples 120
Texture submission: mean 0.027 ms | p95 0.045 ms | samples 120

### Google Chrome

- Copied image visually verified: yes
- Earlier Slice 1–4 scenes regression checked: yes
Core execution: mean 1.809 ms | p95 2.700 ms | samples 120
Pixel conversion: mean 0.196 ms | p95 0.300 ms | samples 120
Texture submission: mean 0.025 ms | p95 0.100 ms | samples 120

### Brave

- Copied image visually verified: yes
- Earlier Slice 1–4 scenes regression checked: yes
Core execution: mean 2.132 ms | p95 2.800 ms | samples 120
Pixel conversion: mean 0.239 ms | p95 0.300 ms | samples 120
Texture submission: mean 0.037 ms | p95 0.100 ms | samples 120


## update a counter through ARM and Thumb status

The original bitmap-copy fixture copies a 38,400-byte indexed image through an
EWRAM mirror and redraws it from canonical EWRAM through a called ARM routine.
The routine saves registers and LR on a mirrored IWRAM stack and returns by
loading PC through LDM. The guest verifies stack restoration before publishing
completion.

### Headless

- Fixture: `copy.gba`
- SHA-256: `70ae32d6d49cfbbe8d05f66da4fa22a1076838b0753cab0a9827f304cb695306`
- Terminal PC: `0x0800a000`
- Completion mailbox ID: `0x005e`
- Completion result: `1`
- Full 38,400-byte copied-image verification: passed
- Full 240×160 framebuffer verification: passed
- EWRAM mirror verification: passed
- IWRAM/stack mirror verification: passed
- BL/routine return verification: passed
- SP restoration verification: passed
- Unaligned STR align-down verification: passed
- Unaligned LDR rotation verification: passed
- Palette byte-write duplication: passed
- BG VRAM byte-write duplication: passed
- OBJ VRAM byte-write ignore behavior: passed
- Guest execution cycles: `1687892`
- Copy/redraw completion host time: TODO ms

### Native Linux

- Copied image visually verified: yes
- Earlier Slice 1–4 scenes regression checked: yes
Core execution: mean 2.099 ms | p95 3.338 ms | samples 120
Pixel conversion: mean 0.099 ms | p95 0.155 ms | samples 119
Texture submission: mean 0.023 ms | p95 0.036 ms | samples 120

### Google Chrome

- Copied image visually verified: yes
- Earlier Slice 1–4 scenes regression checked: yes
Core execution: mean 3.390 ms | p95 4.200 ms | samples 120
Pixel conversion: mean 0.248 ms | p95 0.400 ms | samples 120
Texture submission: mean 0.036 ms | p95 0.100 ms | samples 120

### Brave

- Copied image visually verified: yes
- Earlier Slice 1–4 scenes regression checked: yes
Core execution: mean 3.421 ms | p95 4.000 ms | samples 120
Pixel conversion: mean 0.258 ms | p95 0.400 ms | samples 120
Texture submission: mean 0.027 ms | p95 0.100 ms | samples 120