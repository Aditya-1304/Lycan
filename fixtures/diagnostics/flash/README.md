# Flash64 score cartridge

Original MIT guest assembly reuses the SRAM score scene with Flash commands.
Use `python3 fixtures/diagnostics/flash/build.py` from the workspace root to rebuild and verify
the frozen ROM identity in `manifest.json`. No BIOS is required for this scene.

The guest validates Panasonic identification bytes `32 1b`, exits identification
mode, and reads its score before drawing. Tap and release A (Z in the app) to
add a green eight-by-eight cell, up to sixteen. Updating the two-byte score record
erases its 4 KiB sector and programs the marker and score using standard unlock
commands. The mailbox is `0x03000000 = 0x74`; the score is at `0x03000002`.

`contract.rs` checks three bounded display frames, the source-defined polling
PC range, the mailbox, every completed framebuffer pixel, and all backup bytes.
It verifies restoration and the one-to-two update that needs an erase.
`python3 fixtures/diagnostics/flash/verify.py` executes this contract and the pinned upstream
Flash64 diagnostic in the production WASM core through Node. It does not test
browser IndexedDB or UI behavior.

The separate-process native save probe uses production disk/envelope functions:

```bash
GBA_SAVE_DIR=/tmp/gba-flash-probe cargo run --locked -p gba-app --release -- --save-probe flash-write
GBA_SAVE_DIR=/tmp/gba-flash-probe cargo run --locked -p gba-app --release -- --save-probe flash-read
```

The write command requires a fresh directory. Browser and native GUI checks and
measurement fields are recorded in `docs/performance.md`.
