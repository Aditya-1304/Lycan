# Sprite scene over a scrolling background

The shared Linux/WASM app starts with `sprites.gba`. Arrow keys move a normal
16×16 4bpp player and scroll the mode-0 background once per VBlank. The player
clamps to the visible screen; background scroll wraps modulo 512. Hold Z (A)
to give the player BG priority 1, then release to restore priority 0. Hold X (B)
for 2D OBJ tile mapping; release for 1D. The 2D layout deliberately swaps the
bottom blue/yellow quadrants so a wrong tile-row stride changes the image.

The guest disables all unused OAM entries, writes OBJ palette bank 3, initializes
character data, and updates OAM0 and scroll registers. A stationary magenta
OAM1 object at (120,80) exposes transparent edges and overlap order. The host
supplies logical input and presents completed frames; it does not write OAM.

## Plan acceptance audit

| Slice 9 requirement | Implementation / evidence |
| --- | --- |
| Player controls a sprite across the scrolling scene | Default sprite scene, Load sprite demo and Replay sprite input on both build targets |
| OAM reads and selected sprite layout | Mirrored 1 KiB OAM backing; normal 16×16 4bpp objects with 1D/2D addressing verified |
| Palette handling and transparent pixels | Separate OBJ bank 3, four player colors, magenta overlap marker, transparent player border; exact frame comparisons |
| Priority and overlaps | BG/OBJ ties favor OBJ; lower OAM index owns opaque overlaps before BG-priority comparison; behind-BG capture verifies no OAM1 leak |
| Basic object addressing | Nonzero tile base 4; 1D rows 4..7, 2D bottom row 36..37; deliberately distinct 2D colors |
| OAM access rules required by memory.gba | Mirrored halfword/word reads/writes, ignored byte writes; complete pinned upstream memory diagnostic passes |
| Scripted positions and overlap captures | Five checkpoints, all 38,400 pixels each; OAM, mailbox, BG scroll, layout and frame generations checked |
| Applicable memory fixture passes | Full 13-case memory.gba diagnostic, R12=0, terminal opcode/status and independent success-image digest pass |
| Missing sprite layout behavior recorded | Fixture extensions and unsupported affine layout listed below; no later-slice implementation |
| Native/Chrome/Brave visual acceptance | Not verified; commands and expected checks provided below |

OBJ ordering and BG/OBJ tie handling follow the GBA priority rules documented
in [GBATEK](https://mgba-emu.github.io/gbatek/#lcd-obj---oam-attributes).
The renderer keeps BG priority alongside scanout color and selects the first
opaque OBJ texel in OAM order before comparing it to the winning BG. It uses
stack arrays and the existing frame buffers; no per-scanline heap allocation
or tile cache was introduced.

## Focused RED → GREEN evidence

Realistic bugs caught by these fixtures, absent from earlier bitmap/background
checks: an unmapped OAM bus, wrong OBJ palette region or tile stride, opaque
index-zero texels, incorrect object/background priority, or OAM1 showing through
an opaque OAM0 pixel hidden by BG0. No duplicate unit tests were added.

- Before bus changes, the pinned upstream memory diagnostic failed case 4
  (128 KiB VRAM mirror). Its subsequent OAM checks require the same bus work.
- Before bus changes, the sprite fixture failed writing 0x07000000.
- After bus changes but before OBJ rendering, frame 10 failed at pixel (113,73):
  expected red 0x001f, got background color 0x01e7.
- With OBJ rendering, the same scripted fixture passes every pixel comparison.

| Frame | Guest player | Published player | Guest / image BG priority | Guest / image mapping |
| --- | --- | --- | --- | --- |
| 10 | (112,72) | (112,72) | 0 / 0 | 1D / 1D |
| 13 | (118,72) | (116,72) | 0 / 0 | 1D / 1D |
| 15 | (118,76) | (118,74) | 1 / 1 | 1D / 1D |
| 18 | (116,74) | (116,76) | 0 / 0 | 2D / 2D |
| 20 | (116,74) | (116,74) | 0 / 0 | 1D / 1D |

VBlank publishes the preceding drawing before guest updates. Setup reaches
0x08000220 at 425,827 cycles. Mailbox 0x0062 reports update counts
9,12,14,17,19; final BG scroll and published scroll are both (2,0).
The replay uses 5,617,935 cycles / 491,556 instructions, below its shared
6,000,000-cycle / 1,500,000-instruction bounds. The manifest freezes the original
1,048-byte ROM SHA-256 and independently specifies the app's input timeline.

## Pinned memory diagnostic

The original `memory.gba`, top-level source, and both included test sources are
retained from gba-tests revision `a7113b67e63f83a9b321696ddd7042ccfad6c881`:
[upstream source](https://github.com/jsmolka/gba-tests/blob/a7113b67e63f83a9b321696ddd7042ccfad6c881/memory/memory.asm).
The existing MIT notice covers the retained upstream files. SHA-256 is
`21024fb6aae6343f5f0466dd54e3149de1fbeb23f78e7d85a015c983684d2f87`.

All 13 cases (1..8 and 50..54) execute: RAM/palette/VRAM/OAM/GamePak mirrors,
ignored OAM/OBJ byte stores, and duplicated palette/BG byte stores. VRAM now
mirrors every 128 KiB, with physical 64..96 KiB also mapped at 96..128 KiB.
OAM repeats every 1 KiB, ignores byte stores, and uses one bus cycle for a
halfword or word access. Scanout synchronizes before writes through these paths.

Success requires the exact idle instruction at 0x080004c8, R12=0,
CPSR=0x600000df, frozen original SWI-division firmware, and the independent
success-text framebuffer digest
`59ce42abae9825c2d2579c5cd838e47d88be917e37ea36ff162d46fc5d0991e3`.
Terminal execution takes 244,787 cycles / 22,539 instructions. The original
firmware is only the existing controlled diagnostic service; no IRQ or supplied
BIOS boot path was introduced.

## Layout follow-ups tied to retained fixtures

The verified scene is normal, square 16×16, 4bpp, unflipped, with both mappings.
The size table also decodes normal square/wide/tall sizes and the sampler handles
flips and 8bpp; these additional combinations have no retained capture evidence.

| Follow-up | Fixture to extend / required evidence |
| --- | --- |
| Other normal sizes, rectangular shapes, and flips | Extend sprites.gba with asymmetric texels and exact edge/clipping captures for each selected layout |
| 8bpp and odd tile-base behavior | Extend sprites.gba with independent 256-color OBJ data and 1D/2D captures; current 4bpp evidence does not establish this behavior |
| Affine / double-size objects are currently skipped | Extend sprites.gba with an identity OAM matrix and affine enable, then a transformed/double-size capture in the later affine-sprite slice |
| Semi-transparent / OBJ-window modes are currently skipped; mosaic is not applied | Extend sprites.gba with explicit blend/window/mosaic observations in the corresponding later display work |
| OBJ fetch budget, HBlank arbitration, and active-display bus contention | Extend memory.gba evidence with a timing-specific guest fixture; the current functional memory cases do not establish these timings |

These are recorded limits, not additional Slice 9 work. The selected layout and
applicable upstream diagnostic pass; exhaustive sprite conformance is not claimed.

## Automated validation and profiling

- All 14 existing core/session tests pass; no new unit tests added.
- All earlier fixtures, the full upstream memory diagnostic and five sprite
  captures pass, including bitmap and tiled-background regressions.
- Original fixtures rebuild with frozen hashes; upstream binary identity checks pass.
- Workspace Clippy with warnings denied, native/WASM app checks, and Trunk release build pass.
- A 600-frame optimized headless benchmark completes and rechecks the final image.
- Callgrind completed 60 measured frames plus setup/replay, recording
  2,047,673,889 host instruction references. The source report attributes
  163,010 to OBJ tile-number calculation, 279,220 to OBJ palette reads,
  111,688 to winner storage, and 55,844 to BG-priority comparison.

Instruction references are host work, not GBA cycles or elapsed milliseconds.
The CLI/core timing samples and app callback samples have different boundaries.
Profiled timings must not be used as reference-device elapsed-time measurements.

## Commands for verification and manual measurements

```sh
# Run from the repository root.
mkdir -p /tmp/gba-sprite-results
cargo run --locked -p gba-tools -- build-fixtures
cargo test --locked -p gba-core -p gba-session
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked -p gba-app
cargo check --locked -p gba-app --target wasm32-unknown-unknown
env -u NO_COLOR trunk --config web/Trunk.toml build --release
cargo run --locked -p gba-tools --release -- fixtures run --capture /tmp/gba-sprite-results/frame | tee /tmp/gba-sprite-results/fixtures.txt
cargo run --locked -p gba-tools --release -- bench --scenario sprites --frames 600 | tee /tmp/gba-sprite-results/headless-benchmark.txt
```

For just the new fixtures:

```sh
cargo run --locked -p gba-tools --release -- fixtures run --fixture sprites --capture /tmp/gba-sprite-results/frame
cargo run --locked -p gba-tools --release -- fixtures run --fixture memory --capture /tmp/gba-sprite-results/frame
```

Reference captures are `frame.sprites-frame-{10,13,15,18,20}.ppm` and
`frame.memory.ppm`. The benchmark starts from the settled frame-20 replay,
with released buttons, excludes setup/checking from timing, and checks the
final image again. Mean and p95 each cover 600 complete emulated frame advances.

Native interaction:

```sh
cargo run --locked -p gba-app --release
```

Native automatic capture, in a separate run. Keep the default sprite scene
running and focused, without starting the short replay, until the app closes:

```sh
env GBA_CAPTURE_PATH=/tmp/gba-sprite-results/native.ppm cargo run --locked -p gba-app --release
cat /tmp/gba-sprite-results/native.txt
```

This captures the rendered viewport and the app's 120-sample core/conversion/
texture-submission measurements. Texture submission is CPU queue/copy work,
not GPU completion. Copy those milliseconds into [performance.md](performance.md).

Serve the browser app in another terminal:

```sh
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open each browser manually, separately:

```sh
google-chrome-stable http://127.0.0.1:8080
```

```sh
brave http://127.0.0.1:8080
```

On each platform check arrows and screen clamps, transparent borders over BG and
the magenta marker, Z behind-BG behavior, and X changing only the bottom tile
colors. Run **Replay sprite input**, compare against frame 20, and verify final
player (116,74) / scroll (2,0). Replay pauses; Resume before collecting timing.
Use **Load memory diagnostic** and verify “All tests passed”. Check Reset,
pause/resume, focus loss, and browser hidden-tab restore. Earlier demos remain
available through their existing controls. Open **Performance** after 120 samples
and copy only its mean/p95 milliseconds. Any interaction outcome without evidence
remains marked not verified rather than being inferred from timing values.

To repeat source profiling (requires Valgrind/Callgrind):

```sh
env CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --locked -p gba-tools --release
valgrind --tool=callgrind --callgrind-out-file=/tmp/gba-sprite-results/scanline.callgrind target/release/gba-tools bench --scenario sprites --frames 60
callgrind_annotate --auto=no /tmp/gba-sprite-results/scanline.callgrind crates/gba-core/src/lib.rs > /tmp/gba-sprite-results/scanline-profile.txt
```

The assistant fills non-timing results/statuses. Only millisecond timing fields
are left for manual entry. Linux/Chrome/Brave visual acceptance remains unverified;
no native or browser UI was launched for these automated checks.
