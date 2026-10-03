# Banked Flash128 score cartridge

Original MIT guest assembly selects Sanyo Flash128 through the centralized
`FLASH1M_V103` detector. It checks identification `62 13`, writes bank-zero
`a5 score` and bank-one `5a (score + 16)` at the same chip offsets, and restores
bank zero after accessing bank one. Tap/release A (Z in the app) to increment.
The first green band reads bank zero; the second blue band reads bank one.

`manifest.json` freezes the ROM identity, direct ARM startup, mailboxes, budget,
and source-defined checkpoint ranges. A display boundary can interrupt polling
or drawing. `contract.rs` checks all 38,400 completed pixels and all 131,072 save
bytes after three frames, including a clean reopen and erase/reprogram update.
It also checks rejected short imports and stale revision acknowledgments.

```bash
python3 fixtures/diagnostics/flash-banked/build.py
python3 fixtures/diagnostics/flash-banked/verify.py
GBA_SAVE_DIR=/tmp/gba-banked-probe cargo run --locked -p gba-app --release -- --save-probe banked-write
GBA_SAVE_DIR=/tmp/gba-banked-probe cargo run --locked -p gba-app --release -- --save-probe banked-read
```

Use a fresh directory for the write probe. These separate native processes use
the production disk and identity-bearing export route. The WASM harness runs the
same score contract and unchanged pinned upstream `flash128.gba`; it does not
verify browser IndexedDB. Browser acceptance remains in `docs/performance.md`.
