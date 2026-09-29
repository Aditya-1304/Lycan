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

## Slice 1 verification status

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