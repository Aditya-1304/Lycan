# Slice 3: palette colors and bitmap pages

Implemented against `../../plan_final.md`, Slice 3. Headless behavior and native/WASM
compilation are verified. Linux, Chrome, and Brave visual acceptance remains for
manual verification; no live app or browser was operated during this slice.

## Implementation and scope

- The timed bus maps 1 KiB of palette RAM, including its region mirrors. Halfword
  writes and halfword/word reads share the existing access timing. Reset clears
  palette state along with the rest of the machine.
- Mode 4 resolves one byte per pixel through the BG palette. DISPCNT bit 4 selects
  page offsets 0 or `0xa000`. Index zero uses palette entry zero, and color bit 15
  is masked from framebuffer output. Mode 3 continues to ignore the page bit.
- Palette, VRAM, and display-control writes synchronize scanout before mutation.
  Drawing retains the earlier scanline approximation; finer raster effects are
  deferred to their planned slice.
- `palette.gba` writes distinct packed index pairs to both pages and polls VBlank.
  Holding A selects page 1; holding B changes its second color without rewriting
  VRAM. The app's **Load palette demo** button uses the existing shared session.
- The pinned upstream `hello.gba` additionally requires BL, stack/glyph block
  transfers, word display-status loads, register-offset STRH, immediate register
  shifts, AND/register ALU operands, and MLA. Those forms run through ordinary
  CPU decoding and the timed bus. This does not complete the later CPU diagnostic
  or memory-copy slices. Thumb, exception returns, register-controlled shifts,
  byte transfers, and other unexercised instruction forms remain deferred.

## Focused red/green evidence

The realistic missing behavior was palette writes and indexed/page rendering:
existing mode-3 fixtures never access palette RAM or select a bitmap page. The
original guest first failed with `unmapped 2-byte access at 0x05000000`; after
implementation, all three independently frozen full-frame expectations passed.
The reference guest first failed on BL `0xeb000027` at `0x080000c0`, then passed
after implementing the text routine's required instruction forms. No additional
unit tests were added.

| Capture | Guest state | Every even pixel | Every odd pixel |
| --- | --- | --- | --- |
| Frame 4 | Page 0, original palette | `0x001f` red | `0x03e0` green |
| Frame 6 | Page 1, original palette | `0x7c00` blue | `0x7fff` white |
| Frame 8 | Page 1, changed palette | `0x7c00` blue | `0x001f` red |

The runner checks all 38,400 pixels and the guest mailbox at each checkpoint.
Initialization fills both pages before frame 4. A is pressed at cycle 1,123,584
and B at cycle 1,685,376. Each capture allows a subsequent visible scanout after
the VBlank state change. Captures are RGB PPM files.

```bash
cargo run --locked -p gba-tools -- build-fixtures
cargo run --locked -p gba-tools --release -- fixtures run --capture /tmp/slice3
```

Outputs include `/tmp/slice3.palette-frame-4.ppm`,
`/tmp/slice3.palette-frame-6.ppm`, `/tmp/slice3.palette-frame-8.ppm`, and
`/tmp/slice3.hello.ppm`.

The reference fixture is the unmodified MIT binary from
[jsmolka/gba-tests](https://github.com/jsmolka/gba-tests/tree/a7113b67e63f83a9b321696ddd7042ccfad6c881/ppu),
revision `a7113b67e63f83a9b321696ddd7042ccfad6c881`. Its notice is retained in
`roms/gba-tests/LICENSE`. Its source-defined idle instruction is `0x08000160`.
It uses controlled ARM startup with no BIOS; it is not a BIOS boot check.
`build-fixtures` verifies its frozen binary identity rather than passing FASMARM
source to GNU binutils. Rebuilding upstream requires that revision's complete
`lib/` directory and FASMARM.

The expected hello digest was derived independently from the pinned binary's
64-bit glyph table at file offset `0x214`: character entry `(character - 32) * 8`,
least-significant bit first, eight pixels per row. The source draws
`Hello world!` at (72, 76) with palette entry 1 white and backdrop black. The
frozen expected SHA-256 covers row-major little-endian BGR555 halfwords:
`56cd131fb3915fe7e410be228a8c09e99132064799f148583636ca75745bedf7`.
The runner reaches the terminal instruction and waits for a complete subsequent
visible frame, ending at cycle 561,807 within its 1,000,000-cycle bound.

## Checks run

- Original fixture rebuilds match frozen hashes; upstream hello identity matches.
- Release fixture runner passes pixels, guest-store mutation, all five movement
  checkpoints, all three palette checkpoints, and hello completion/image digest.
- Existing core/session tests: 11 passed, 0 failed.
- Formatting, changed-crate Clippy with warnings denied, native app compilation,
  WASM app compilation, and diff whitespace checks pass.

## Manual application acceptance

Run the Linux app and repeat in Chrome and Brave using the browser build:

1. Select **Load palette demo** and allow initialization to finish. Expect
   alternating red/green columns across the entire 240-by-160 image.
2. Hold Z (A). Expect blue/white columns, matching frame 6.
3. Keep Z held and also hold X (B). Expect blue/red columns, matching frame 8.
4. Release X and then Z. Expect the original page colors to return. Check reset,
   pause/resume, and focus-loss release with the demo.
5. Drop `roms/gba-tests/hello.gba`. Expect white `Hello world!` at (72, 76)
   on black, matching the headless capture.
6. Rerun the existing square demo and pixel ROM visually.

Slice 3's implementation and headless checks are ready. Its plan completion
condition remains open until the displayed images match these captures on Linux,
Chrome, and Brave. This slice does not establish browser runtime or performance
acceptance from compilation alone.
