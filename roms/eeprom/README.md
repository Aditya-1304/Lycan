# EEPROM score cartridges

Original guest assembly uses `EEPROM_V124` for family identification and DMA3
halfword command lengths for automatic capacity detection. The same source is
assembled with six address bits for `eeprom512-score.gba` and fourteen for
`eeprom8k-score.gba`. The first physical block holds `5a (score + 16)`; the last
holds `a5 score`. The remaining six bytes of each record are zero. All untouched
bytes remain erased (`ff`). Tap/release A (Z in the app) to increment the score.
Green and blue bands independently render the last and first records.

Protocol reference: [GBATEK EEPROM](https://mgba-emu.github.io/gbatek/#gbacartbackupeeprom).

Read commands transfer 9/17 halfwords; writes transfer 73/81. Reads decode four
dummy clocks and 64 data clocks. Writes poll ready and verify serial readback.
The guest supplies every score value and pixel; the host only provides initial
backup bytes and button transitions.

`manifest.json` freezes both ROM hashes, startup, mailbox identity, bounded
execution, source-defined checkpoint ranges and completed image expectations.
`contract.rs` checks all 38,400 pixels, every backup byte, clean restore, the next
update, capacity overrides, rejected imports and stale acknowledgments. Its
production WASM harness runs headlessly in Node; it does not verify IndexedDB.

```bash
python3 roms/eeprom/build.py
python3 roms/eeprom/verify.py
GBA_SAVE_DIR=/tmp/gba-eeprom-probe cargo run --locked -p gba-app --release -- --save-probe eeprom512-write
GBA_SAVE_DIR=/tmp/gba-eeprom-probe cargo run --locked -p gba-app --release -- --save-probe eeprom512-read
GBA_SAVE_DIR=/tmp/gba-eeprom-probe cargo run --locked -p gba-app --release -- --save-probe eeprom8k-write
GBA_SAVE_DIR=/tmp/gba-eeprom-probe cargo run --locked -p gba-app --release -- --save-probe eeprom8k-read
```

Use a fresh probe directory for writes. Separate processes use production disk
storage and identity-bearing exports. Browser acceptance is documented in
`docs/performance.md`. Core programming completes synchronously; physical EEPROM
busy duration is not modeled, matching the current Flash implementation.
