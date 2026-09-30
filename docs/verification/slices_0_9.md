# Slice 0–9 verification and remediation record

Record date: 2026-09-30. Specification: `plan_final.md`, Slice 0–9 acceptance
criteria and Section 6. Scope: verification, CI and documentation hardening;
no Slice 10 implementation, emulator architecture changes, manifest changes,
ROM changes or optimizations. The worktree was clean at the start.

## Remediated issues

| Issue | Change |
| --- | --- |
| Referenced WASM verifier absent | Added `roms/cartridge/verify_wasm.py`, `verify_wasm.mjs`, and `wasm_harness.rs`; execution uses compiled production gba-core in Node WebAssembly |
| Native-only timing acceptance | All 24 manifest intervals now execute in WASM with exact cycles, WAITCNT, guest reports, terminal PC, completion/data mailbox and shared total budgets |
| Limited CI binary diff | `.github/workflows/ci.yml` now runs `git diff --exit-code -- roms` immediately after the builder in the same step; existing frozen hash checks retained |
| Incomplete cross-target diagnostic evidence | Optional `--diagnostics` also runs ARM, Thumb, services and memory in WASM; CI enables it |
| Mixed performance and functional records | `docs/performance.md` now contains benchmark tables, environment metadata, measurement provenance and target comparisons; functional evidence is indexed here |
| Counter heading repeated copy text | Removed duplicated copy evidence; retained timings with unresolved ROM attribution, without asserting they measure the counter |
| Corrupted Chrome Slice 3 text | Removed embedded placeholders; preserved 2.938/3.800, 0.234/0.300 and 0.032/0.100 ms with 120 samples per metric |
| Slice 9 timing contradictions | Retained existing app measurements; removed obsolete “not collected” claims; corrected only the truncated Chrome conversion unit `m` to `ms` |
| Obsolete paths and implementation claims | Replaced local absolute cwd commands in diagnostic/tiled/sprite docs; corrected tiled default-scene and counter WAITCNT statements; corrected cartridge README's absent-upstream claim |

## New runtime verification and failure-first evidence

Realistic bug caught by the new check: a WASM-specific execution, timing or
completion regression can pass every native Rust fixture test. The existing
WASM compile check does not execute a guest and cannot catch that regression.
The required command initially failed because `verify_wasm.py` did not exist.
After adding the driver, the compiled-WASM guest passed all declared intervals.
No artificial failing assertion or altered expected cycle count was introduced.

The Python driver parses the existing TOML manifest, checks ROM identity, builds
`gba-core` with locked release Cargo for `wasm32-unknown-unknown`, and locates its
rlib from Cargo's artifact record. Rust links a temporary cdylib bridge to that
rlib. Node compiles and instantiates the resulting WASM; no host imports or native
fallback are allowed. The bridge keeps one owned machine across the whole timing
sequence. Every marker gets the remaining instruction budget and absolute cycle
limit. Rust traps or Node assertion failures produce nonzero process status.
No expected cycle values are maintained outside the manifest.

The extra diagnostic runtime catches a separate target-specific behavior gap:
CPU status, test-firmware exception/return execution, result registers or rendered
success text could regress in WASM while the cartridge timing guest still passes.
It reuses the existing guest assertions and frozen diagnostic contracts, including
firmware hash, terminal opcode, total budgets and complete success-image digest.
No new emulator behavior or duplicate native unit tests were added.

Run from the repository root:

```sh
python3 roms/cartridge/verify_wasm.py
python3 roms/cartridge/verify_wasm.py --diagnostics
```

Requirements: Python 3.11+, Node 26.10.0, pinned Rust and its WASM target. Locally
executed with Python 3.14.7 and Node v26.10.0; CI pins the same Node version. Both commands
passed locally. Node is a core runtime check, not a Chrome/Brave UI observation.

## Native versus WASM timing results

All values are measured emulated cycles and equal the frozen manifest values.
Each row represents two scenarios, for 24 total. ARM/Thumb identify 32/16-bit
fetch cases, not wall-clock benchmark measurements.

| WAITCNT | Window | Frozen ARM / Thumb | Native ARM / Thumb | WASM ARM / Thumb |
| --- | --- | --- | --- | --- |
| 0x0000 | WS0 | 121 / 88 | 121 / 88 | 121 / 88 |
| 0x0000 | WS1 | 155 / 100 | 155 / 100 | 155 / 100 |
| 0x0000 | WS2 | 223 / 124 | 223 / 124 | 223 / 124 |
| 0x06da | WS0 | 88 / 66 | 88 / 66 | 88 / 66 |
| 0x06da | WS1 | 88 / 66 | 88 / 66 | 88 / 66 |
| 0x06da | WS2 | 88 / 66 | 88 / 66 | 88 / 66 |
| 0x4000 | WS0 | 85 / 64 | 85 / 64 | 85 / 64 |
| 0x4000 | WS1 | 127 / 72 | 127 / 72 | 127 / 72 |
| 0x4000 | WS2 | 211 / 112 | 211 / 112 | 211 / 112 |
| 0x46da | WS0 | 60 / 54 | 60 / 54 | 60 / 54 |
| 0x46da | WS1 | 60 / 54 | 60 / 54 | 60 / 54 |
| 0x46da | WS2 | 60 / 54 | 60 / 54 | 60 / 54 |

Both targets passed terminal PC `0x08002018`, mailbox ID `0x0060`, result `1`,
loaded-data halves `0x5678`/`0x1234`, all 24 guest timing reports, and the shared
2,000-instruction / 20,000-cycle limits. WASM completed at 6,513 cycles.
WAITCNT 0x0000/0x06da disable prefetch; 0x4000/0x46da enable it.

## Fixture reproducibility

The builder passed before source/documentation edits. Immediately afterward,
`git diff --exit-code -- roms` passed. The builder checks frozen hashes before
writing original artifacts and directly verifies retained upstream identities.
An inventory comparison confirmed every tracked `.gba`/`.bin` artifact is
represented by a manifest fixture or the diagnostic firmware contract.
No manifest hashes, expected outputs or tracked generated binaries changed.

| Artifact group | Covered paths | Result |
| --- | --- | --- |
| Original guest ROMs (10) | `roms/{pixels,buttons,palette,calculations,copy,counter,cartridge,tiled,sprites}.gba`, `roms/test-firmware/services.gba` | Rebuilt, frozen SHA-256 matched, binary diff clean |
| Original test firmware (1) | `roms/test-firmware/division.bin` | Rebuilt; all diagnostic firmware hashes matched; binary diff clean |
| Retained upstream ROMs (5) | `roms/gba-tests/hello.gba`, `stripes.gba`, `arm/arm.gba`, `thumb/thumb.gba`, `memory/memory.gba` | Frozen SHA-256 identity checks passed |

CI now checks the entire tracked `roms` tree after rebuilding, including all
later artifacts. Existing core tests, session tests, headless fixtures, Clippy,
WASM compilation and Trunk build steps remain; workspace tests and the WASM
runtime check are additional gates. GitHub-hosted execution has not been observed
in this local remediation; results below are local execution evidence.

## Slice-by-slice acceptance evidence

| Slice | Automated evidence rechecked | App acceptance / details |
| --- | --- | --- |
| 0 | Native/WASM checks and Trunk release build; session input/pause/reset checks | Historical presentation assertions retained below; Slice 0 test-image provider has been replaced |
| 1 | All 38,400 pixels, mailbox, terminal bounds and guest-store mutation | Current Linux/Chrome/Brave visual recheck pending |
| 2 | Five guest/published positions and complete frames; 60/144 Hz equality, replay deadline, pause/focus re-anchor | Historical user-recorded visual checks retained below; fresh recheck pending; [details](../button_movement.md) |
| 3 | Both mode-4 pages, palette-only update, full frames and pinned hello digest | Historical visual checks retained below; fresh recheck pending; [details](../palette_pages.md) |
| 4 | All 16 guest cases, first-failure mailbox and full result image | Native passed; browser app timings are historical evidence, fresh visual recheck pending |
| 5 | Copied bytes, image, mirror/alignment/stack/return checks | Historical visual checks retained below; fresh recheck pending; [details](../copy-demo.md) |
| 5A | 24 exact intervals, completion mailbox and budgets in native and WASM | Passed for retained sequences; [scope](../../roms/cartridge/README.md) |
| 6 | Six counter checkpoints, ARM return and architectural Thumb PC observations, full images | Counter UI recheck pending; [details](../thumb-counter.md) |
| 7 | Full ARM/Thumb diagnostics and original services guest; exact result/status/terminal and success-image digests in native and WASM | Actual app display/reset interaction pending; [details](../cpu_diagnostics.md) |
| 8 | Six scroll/wrap captures, frozen update counts/registers/generations, all pixels; stripes reference; prior bitmap fixtures | Linux/Chrome/Brave visual scroll/wrap recheck pending; [details](../tiled_background.md) |
| 9 | Five sprite captures, movement, transparency, overlap/OAM order, BG priority and distinct 1D/2D colors; full memory diagnostic | Linux/Chrome/Brave visual and interaction checks pending; [details](../sprite_scene.md) |

### Diagnostic completion and result presentation

| Guest | Terminal PC | Result | CPSR | Native / WASM terminal cycles | Native / WASM instructions | Success frame digest |
| --- | --- | --- | --- | --- | --- | --- |
| ARM | 0x08001ec4 | R12=0 | 0x6000001f | 244803 / 244803 | 22887 / 22887 | Passed in both |
| Thumb | 0x08000aac | R7=0 | 0x600000df | 244771 / 244771 | 22944 / 22944 | Passed in both |
| Services | 0x08000400 | R12=0 | 0x600000df | 1513 / 1513 | 529 / 529 | Not declared; service results checked |
| Memory | 0x080004c8 | R12=0 | 0x600000df | 244787 / 244787 | 22539 / 22539 | Passed in both |

All four use manifest limits of 400,000 instructions / 3,000,000 cycles and the
frozen original division firmware. The three success screens match the manifest
SHA-256 `59ce42abae9825c2d2579c5cd838e47d88be917e37ea36ff162d46fc5d0991e3`.
This proves completed guest scanout, not texture presentation in an application.
All 13 memory cases run, including OAM mirroring and ignored byte stores.
Previous ARM/Thumb case-1 failure injection and full `Failed test 001` captures
remain documented in cpu_diagnostics.md; that injection was not rerun here.

### Scroll and sprite checkpoint evidence

Tiled frames 10/11/13/15/18/19 all passed all 38,400 pixels, including both wrap
boundaries. Final guest/published scroll is (2,0); replay totals remain 5,337,027
cycles / 467,886 instructions. Stripes reaches 0x08000140 and its exact 8-pixel
vertical stripe image passes. Bitmap regression outputs remain unchanged.

Sprite frames 10/13/15/18/20 all passed all 38,400 pixels. Final guest/published
player is (116,74), scroll (2,0); replay totals remain 5,617,935 cycles / 491,556
instructions. Behind-BG and OAM overlap ordering, transparent borders, OBJ palette
bank 3, and deliberately different 2D bottom-row colors pass. The final image is
rechecked after each headless benchmark. Additional sizes/shapes/flips/8bpp and
unimplemented later display modes remain fixture-tied follow-ups in sprite_scene.md.

The pinned stripes loader still appends two unexecuted look-ahead words; general
absent-ROM bus behavior is a recorded limitation in tiled_background.md. No
hardware-equivalent absent-ROM read claim is added by this closeout.

## Historical visual evidence retained from performance.md

These are prior recorded statements, not new observations from this remediation.
Timing entries never imply a visual pass. No fresh screenshot or UI observation
was produced. Individual slice documents retain detailed correctness evidence,
fixture identities, RED/GREEN history and user-run instructions.

| Prior record | Native Linux | Chrome | Brave |
| --- | --- | --- | --- |
| Slice 0 test animation, resize/aspect, button mapping, focus loss, pause without busy-spin, reset | Prior document asserted success across targets, but also said manual interaction was unobserved; provenance unresolved | Same unresolved prior claims | Same unresolved prior claims |
| Slice 1 mode-3 image | Blank visual outcome field; no recorded pass | Blank visual outcome field | Blank visual outcome field |
| Slice 2 movement, replay final (114,73), pause, focus loss | Recorded yes; boundary clamps and reset also yes | Recorded yes; hidden-tab restore also yes | Recorded yes; hidden-tab restore also yes |
| Slice 3 pages 0/1, palette-only update, hello, pause/reset/focus, earlier scenes | Recorded yes | Recorded yes; corrupted palette-only text repaired as historical yes | Recorded yes |
| Slice 4 calculation board, all bands, completion/first-failure result, earlier scenes | Recorded yes | Recorded yes | Recorded yes |
| Slice 5 copied image and earlier scenes | Recorded yes | Recorded yes | Recorded yes |
| Counter heading's copied-image claims | Duplicate copy text; not reliable counter acceptance evidence | Same duplicated text | Same duplicated text |
| Slice 7 diagnostic screens and interaction | Pending | Pending | Pending |
| Slice 8 scroll/wrap, stripes, earlier bitmap scenes and interaction | Not verified visually | Not verified visually | Not verified visually |
| Slice 9 movement/clamps, overlap/transparency, priority/mapping, memory screen and interaction | Not verified visually | Not verified visually | Not verified visually |

## Remaining manual platform acceptance

No native GUI or Chrome/Brave was launched or controlled for this audit. This
execution pass used CLI tools and Node WASM. The established project workflow
reserves graphical acceptance to user-performed checks. Headless PPM captures
are guest framebuffer artifacts, not screenshots or evidence of host interaction.

Repeat on Linux, Chrome and Brave and record observations separately:

- Slice 7: load ARM and Thumb tests; observe `All tests passed`, Reset reruns,
  pause/resume, resize and earlier demo displays.
- Slice 8: select Load tiled demo, cross horizontal and vertical wrap boundaries,
  run Replay tiled input, compare the final (2,0) image to frame 19, and load
  stripes reference. Compare the earlier bitmap scenes.
- Slice 9: check arrow movement/clamps, transparent border over the magenta
  marker, Z (guest A) behind-BG priority, X (guest B) 1D/2D bottom-color change,
  and Replay sprite input final player (116,74) / scroll (2,0). Load memory
  diagnostic and observe `All tests passed`.
- Across earlier demos: pause/resume without catch-up, reset, focus-loss release,
  and Chrome/Brave hidden-tab restoration. Check cadence at different monitor
  refresh rates and that paused apps do not busy-spin.
- Section 6: collect actual complete app-frame/GPU timings, focused sustained
  cycles/wall-time speed, 30-minute resource/input-delay trends and allocation
  profiling. Record ARM and Thumb app workloads separately. Identify the ROM
  underlying the counter-heading measurements and the two Slice 8 profile totals.

Commands from the repository root:

```sh
cargo run --locked -p gba-app --release
```

Serve the browser app in another terminal:

```sh
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open Chrome/Brave yourself at `http://127.0.0.1:8080`. Exact per-slice capture and
interaction instructions are linked above. The current default is the sprite
scene; select the tiled demo before collecting tiled evidence. Replay pauses on
completion; resume before measuring. Performance values belong in performance.md;
visual/interaction outcomes belong here or in the individual slice document.

## Final local validation

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| Separate `rustfmt --edition 2024 --check roms/cartridge/wasm_harness.rs` | Passed |
| `cargo run --locked -p gba-tools -- build-fixtures` | Passed, all 16 artifacts covered |
| `git diff --exit-code -- roms`, immediately after initial rebuild and before edits | Passed |
| Final tracked generated-binary diff and frozen-hash inventory | Passed; ROMs/firmware and manifest unchanged |
| `cargo test --locked --workspace` | Passed: 7 core + 7 session tests; app/tools/doc tests also green |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed |
| `cargo run --locked -p gba-tools --release -- fixtures run --manifest fixtures/manifest.toml` | Passed all 15 fixtures, all original expected outputs |
| `cargo check --locked -p gba-app` | Passed |
| `cargo check --locked -p gba-app --target wasm32-unknown-unknown` | Passed |
| Literal Trunk release command | Environment failure: NO_COLOR=1 is invalid for this Trunk CLI |
| `env -u NO_COLOR trunk --config web/Trunk.toml build --release` | Passed |
| New WASM verifier, default and `--diagnostics` | Passed: 24 timing scenarios plus four diagnostic contracts |
| Headless `bench --scenario pixels\|tiled\|sprites --frames 600` | Each scenario passed; results in performance.md |
| Native / Chrome / Brave fresh graphical interaction | Not executed; user-performed acceptance remains pending |

Headless captures were regenerated under `/tmp/gba-closeout`, including diagnostic,
scroll/wrap and sprite frames. They are temporary and reproducible, not durable
platform screenshots. Profiling records are historical; no new Callgrind or
allocation-profile run was needed for this verification-only change.

Historical timing numbers, precision, sample counts and available provenance
were preserved. A comparison against the pre-edit performance record checked
all 182 historical numeric timing occurrences for retained values; structured
platform rows retain the distinct core/conversion/submission sample counts,
including the native counter-heading conversion count of 119. New CLI results
are separately dated and identify the current balanced power profile, so they
do not overwrite the AC/performance reference record.

Automated remediation is complete. Full Slice 0–9 platform/performance closure
is not claimed while the manual and unmeasured Section 6 checks above remain open.
