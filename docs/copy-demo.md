# Bitmap copy demo

The original MIT guest in `roms/copy/copy.s` copies a 38,400-byte indexed
bitmap from cartridge ROM to the EWRAM mirror at `0x02040000`. A called ARM
routine redraws it from canonical EWRAM, saves registers and LR with STMDB,
and returns with LDM loading PC. The caller checks that SP was restored
before publishing completion.

The shared application's **Load copy demo** button loads the same ROM on
Linux and browser hosts. The expected image is an 80-row red upper band and
an 80-row green lower band.

## Plan acceptance

- Immediate word/byte and halfword transfers, BL, and stack/block transfers
  execute in the guest. Existing branch/link and block-transfer support is reused.
- EWRAM and IWRAM mirrors are exercised, including the stack mirror.
- An unaligned STR aligns down; an unaligned LDR rotates the aligned word.
- Palette byte stores and bitmap BG VRAM byte stores duplicate onto the
  halfword write bus. Bitmap OBJ VRAM ignores byte stores; OAM is untouched.
- The runner checks every copied halfword, independent bus-rule probes,
  completion ID `0x005e`, result `1`, terminal PC `0x0800a000`, and all
  38,400 completed-frame pixels under declared instruction/cycle limits.

## Verification

The initial RED run failed on missing `STRB` behavior (`0xe5c01004` at
`0x08000020`). After implementation, the strengthened two-band fixture passes:
`cycles=1687892`, `generation=6`. No additional Rust test files were added.
All earlier fixtures retain their expected outputs.

Rebuild and run headlessly:

```sh
cargo run --locked -p gba-tools -- build-fixtures
cargo run --locked -p gba-tools --release -- fixtures run
```

Linux visual acceptance remains user-performed:

```sh
cargo run --locked -p gba-app
```

Browser visual acceptance remains user-performed in Chrome and Brave:

```sh
env -u NO_COLOR trunk --config web/Trunk.toml serve --release
```

Select **Load copy demo** and verify both bands. Automated fixture evidence
and native/WASM build checks do not establish visual acceptance on these hosts.
WAITCNT/prefetch timing remains outside this change.
