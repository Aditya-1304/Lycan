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

Record later measurements with the build profile, browser, display, power conditions, and tracing state above. Keep native and browser results separate.
