# Mode-0 tiled background

Select **Load tiled demo** in the shared native/WASM application (which now
starts with the sprite scene). Arrow keys change
BG0 scroll by two pixels once per VBlank. Both coordinates wrap modulo 512.
The original MIT guest initializes two asymmetric 4bpp tiles, four palette
banks, and four 32×32 screen blocks. Alternating map entries flip horizontally
and vertically; index-zero texels expose the white backdrop. No host code
updates guest scroll registers or generates the displayed scene.

## Plan acceptance audit

| Slice 8 requirement | Implementation and evidence |
| --- | --- |
| Replace bitmap scene with button-controlled mode 0 | Load tiled demo and Replay tiled input on Linux/WASM; current app default is the sprite scene |
| Tile/palette/map interpretation, dimensions, scrolling, wrapping | Scanline register snapshots; text tile lookup, palette banks, flips, transparency, screen-block addressing and modulo dimensions; 512×512 scene verified completely |
| Guest updates scroll once per frame | VBlank-edge loop; frozen mailbox update counts and BG0HOFS/BG0VOFS checked at six checkpoints |
| Scripted scroll/wrap captures | Each checkpoint compares all 38,400 pixels with an independent world-coordinate oracle; captures emitted as PPM |
| Try gba-tests stripes.gba | Pinned upstream identity, source-defined idle instruction at 0x08000140, all 38,400 alternating-stripe pixels pass |
| Earlier bitmap programs still work | Full existing fixture suite passes, including bitmap modes 3/4 and hello |
| Profile scanline drawing before caches | Callgrind source profile of optimized code obtained; no tile caches introduced |
| Native/Chrome/Brave runtime acceptance | User-performed visual checks pending; recorded app timings are in performance.md |

The renderer also interprets the standard text-background sizes and 8bpp
palette addressing, and composites enabled text backgrounds by priority.
The retained scene evidence specifically exercises 4bpp BG0 at 512×512 and
upstream 4bpp BG0 at 256×256. It does not establish exhaustive PPU conformance.
Sprites, affine backgrounds, windows, blending, mosaic, and within-scanline
register effects are outside this slice.

## RED → GREEN evidence

Realistic bug caught: missing mode-0 rendering, incorrect quadrant addressing,
flips, palette banking, transparency, or wrap offsets can produce a stationary
or incorrectly colored background while every previous bitmap test passes.
The guest fixture is the focused behavior check; no duplicate unit tests were
added. Before the renderer change, the fixture failed at frame 10, pixel (0,0):
expected 0x0421, got the backdrop 0x7fff. The same fixture passes after the change.

| Frame | Guest updates | Guest scroll | Published image scroll |
| --- | --- | --- | --- |
| 10 | 9 | (510,510) | (510,510) |
| 11 | 10 | (0,510) | (510,510) |
| 13 | 12 | (4,510) | (2,510) |
| 15 | 14 | (4,2) | (4,0) |
| 18 | 17 | (2,0) | (2,2) |
| 19 | 18 | (2,0) | (2,0) |

VBlank publishes the preceding visible frame before the guest's new scroll
update. Setup reaches the declared ready PC at 416,681 cycles; early forced-blank
frames are excluded from scene comparisons. Completion uses mailbox 0x0061,
frozen update counts, exact scroll registers, frame generations, full pixels,
and shared budgets. The replay ends at 5,337,027 cycles / 467,886 instructions,
under 6,000,000 cycles / 1,500,000 instructions.

The manifest freezes the original 424-byte ROM hash and the app input timeline
is checked against its separately declared events. The upstream stripes ROM
is the unmodified 324-byte file from gba-tests revision
`a7113b67e63f83a9b321696ddd7042ccfad6c881`:
[upstream source](https://github.com/jsmolka/gba-tests/blob/a7113b67e63f83a9b321696ddd7042ccfad6c881/ppu/stripes.asm).
The harness and app append two unexecuted look-ahead words when loading this
specific pinned file because its idle branch is its last word. Its hash remains
over the original file. General absent-ROM bus reads remain a loader follow-up;
this padding does not claim hardware-equivalent behavior for those reads.

## Automated validation

- Reproducible original fixture builds and pinned upstream identity checks pass.
- All earlier fixtures and both new scenes pass.
- All 14 existing core/session tests pass.
- Workspace Clippy with warnings denied passes.
- Native and WASM app compilation and Trunk release build pass.
- Renderer scanout uses existing buffers and allocates no per-pixel/scanline data.

An optimized Callgrind run (60 measured frames plus the 19-frame setup/replay)
collected 1,877,851,986 host instruction references. Source attribution shows
23,750,400 references each for map offset calculation and map-entry reads,
11,875,200 for packed-nibble extraction, and 31,172,760 for palette color reads.
Inlined rendering is attributed to System::advance_time; these counts describe
host work, not GBA cycles or elapsed milliseconds. No cache optimization was
made. Instrumented benchmark timing must not be entered as real performance.

## Repeatable commands

Run from the worktree:

```sh
# Run from the repository root.
mkdir -p /tmp/gba-tiled-results
cargo run --locked -p gba-tools -- build-fixtures
cargo test --locked -p gba-core -p gba-session
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked -p gba-app
cargo check --locked -p gba-app --target wasm32-unknown-unknown
cargo run --locked -p gba-tools --release -- fixtures run --capture /tmp/gba-tiled-results/frame | tee /tmp/gba-tiled-results/fixtures.txt
cargo run --locked -p gba-tools --release -- bench --scenario tiled --frames 600 | tee /tmp/gba-tiled-results/headless-benchmark.txt
```

The captures include `frame.tiled-frame-10.ppm` through the declared frame 19
checkpoints and `frame.stripes.ppm`. The headless benchmark checks the scripted
scene first, times 600 full emulated frames with released buttons, and verifies
the final image again. Its mean/p95 exclude setup and verification work.
Unthrottled speed is `16.742706 / core_mean_ms` times real time. App Performance
samples instead measure bounded logic callbacks, conversion, and CPU texture
submission; they are not GPU timings or the headless frame samples.

For source profiling (requires Valgrind/Callgrind):

```sh
env CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --locked -p gba-tools --release
valgrind --tool=callgrind --callgrind-out-file=/tmp/gba-tiled-results/scanline.callgrind target/release/gba-tools bench --scenario tiled --frames 60
callgrind_annotate --auto=no /tmp/gba-tiled-results/scanline.callgrind crates/gba-core/src/lib.rs > /tmp/gba-tiled-results/scanline-profile.txt
```

The debug-info override preserves release optimization. Record hot source
lines from `scanline-profile.txt`; obtain ordinary elapsed-time results from the
unprofiled benchmark above.

Native measurements and interaction:

```sh
cargo run --locked -p gba-app --release
```

For an automatic native viewport capture plus a companion text file containing
120 Performance samples, run this separately, select **Load tiled demo**, and leave that scene
playing and focused until the app closes:

```sh
env GBA_CAPTURE_PATH=/tmp/gba-tiled-results/native.ppm cargo run --locked -p gba-app --release
cat /tmp/gba-tiled-results/native.txt
```

For browser measurements, serve in one terminal and open each browser manually:

```sh
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

```sh
google-chrome-stable http://127.0.0.1:8080
```

```sh
brave http://127.0.0.1:8080
```

In Linux, Chrome, and Brave, select **Load tiled demo**, hold each arrow to cross
both wrap boundaries, then select **Replay tiled input** and verify the final
scroll status `(2, 0)` and image against frame 19. Replay pauses on completion;
click Resume before timing. Open **Performance** and copy its three mean/p95
lines after 120 samples. Record moving and idle observations separately. Also
check pause/resume, Reset, focus loss, and browser hidden-tab restore. Load
**stripes reference** and verify alternating 8-pixel vertical stripes. Check
the earlier bitmap scenes through their existing controls or ROM file loading.
For sustained speed, record the displayed GBA cycle counter before/after a
focused wall-time interval: `(end_cycles - start_cycles) / (16777216 * seconds)`.
Record screenshots manually in browsers and fill only measured fields in
[performance.md](performance.md). Browser interaction is not automated here.
