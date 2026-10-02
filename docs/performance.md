# Performance benchmarking record

## Reference hardware and benchmark configuration

| Field | Recorded configuration |
| --- | --- |
| Reference record date | 2026-09-29 |
| CPU | Intel Core i7-13620H, 10 cores / 16 threads |
| OS / desktop | Arch Linux (rolling), Hyprland / Wayland |
| Display | Chimei Innolux 1920×1080, 60.001 Hz, scale 1 |
| Audio device | Built-in Audio Analog Stereo, PipeWire 1.6.8 |
| Power | AC connected, performance profile |
| Rust | rustc 1.98.1 (48a229cea, 2026-09-01); cargo 1.98.1 (797e8a9bc, 2026-08-05) |
| Trunk | 0.21.14 |
| Chrome / Brave | 153.0.8010.47 / 153.1.95.102; checked 2026-09-30 for Slice 7 |
| Build | Release, opt-level 3, thin LTO; WASM target wasm32-unknown-unknown |
| Tracing / extensions | Tracing disabled for benchmarks (absent in Slice 0); no browser extensions reported |

Reference metadata was not independently refreshed for the later measurement
sessions. Measurement-session refresh rate and browser zoom were not recorded.

## Performance targets

| Section 6 target | Threshold |
| --- | --- |
| Sustained emulation | About 100% of 16,777,216 cycles/s |
| Unthrottled execution | At least 1.25× real time; aim for 1.5× |
| Core p95 | Below 12 ms |
| Complete frame budget | 16.7427 ms for 280,896 emulated cycles |
| Audio buffering | 40–80 ms; applicable when audio is implemented |
| Long session | 30 minutes without growing queues, memory use or audio delay |
| Allocation policy | No routine allocation per instruction, memory access, scanline or sample |

## Slice 0–9 performance measurements

Each row reports **mean / p95 in ms**, with sample counts in core / conversion /
submission order. App core samples are bounded callback execution, which can
advance fractions of a frame or multiple frames. Conversion and texture
submission are distinct CPU operations. Submission does not measure GPU
completion. No complete application-frame or GPU/presentation timing was
recorded for any slice. Sustained cycles/wall-time speed and 30-minute resource
trends were not measured on any app platform. Callback timings alone do not
establish these targets.

All historical millisecond entries below retain their recorded precision.
Missing measurements are labeled Not measured; existing values are not replaced
by this remediation's new CLI runs. Functional acceptance evidence is maintained
in [the verification record](verification/slices_0_9.md).

### Slice 0: Platform setup

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples |
| --- | --- | --- | --- | --- |
| Native Linux | Not measured | Not measured | Not measured | — / — / — |
| Google Chrome | Not measured | Not measured | Not measured | — / — / — |
| Brave | Not measured | Not measured | Not measured | — / — / — |

No host benchmark was recorded for this slice.

### Slice 1: ROM pixels

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples |
| --- | --- | --- | --- | --- |
| Native Linux | 0.946 / 1.531 | 0.119 / 0.168 | 0.026 / 0.041 | Not recorded / Not recorded / Not recorded |
| Google Chrome | 1.004 / 1.400 | 0.202 / 0.300 | 0.021 / 0.100 | 120 / 120 / 120 |
| Brave | 1.085 / 1.300 | 0.211 / 0.300 | 0.032 / 0.100 | 120 / 120 / 120 |

Provenance: existing user-entered app record; measurement timestamp and
moving/idle conditions were not recorded.

### Slice 2: Moving square

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples |
| --- | --- | --- | --- | --- |
| Native Linux | 1.828 / 2.369 | 0.128 / 0.168 | 0.026 / 0.036 | 120 / 120 / 120 |
| Google Chrome | 2.359 / 2.900 | 0.230 / 0.300 | 0.031 / 0.100 | 120 / 120 / 120 |
| Brave | 2.463 / 3.100 | 0.243 / 0.300 | 0.031 / 0.100 | 120 / 120 / 120 |

Provenance: existing user-entered app record; measurement timestamp and
moving/idle conditions were not recorded.

### Slice 3: Palette and bitmap pages

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples |
| --- | --- | --- | --- | --- |
| Native Linux | 2.142 / 3.190 | 0.122 / 0.176 | 0.027 / 0.039 | 120 / 120 / 120 |
| Google Chrome | 2.938 / 3.800 | 0.234 / 0.300 | 0.032 / 0.100 | 120 / 120 / 120 |
| Brave | 2.262 / 3.400 | 0.188 / 0.300 | 0.028 / 0.100 | 120 / 120 / 120 |

Provenance: existing user-entered app record; measurement timestamp and
moving/idle conditions were not recorded.

### Slice 4: CPU calculations

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples |
| --- | --- | --- | --- | --- |
| Native Linux | 1.863 / 2.962 | 0.110 / 0.157 | 0.026 / 0.041 | 120 / 120 / 120 |
| Google Chrome | 2.044 / 6.700 | 0.156 / 0.300 | 0.029 / 0.100 | 120 / 120 / 120 |
| Brave | 2.125 / 2.800 | 0.227 / 0.300 | 0.028 / 0.100 | 120 / 120 / 120 |

Provenance: existing user-entered app record; measurement timestamp and
moving/idle conditions were not recorded.

### Slice 5: Bitmap copy

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples |
| --- | --- | --- | --- | --- |
| Native Linux | 2.202 / 3.093 | 0.125 / 0.157 | 0.027 / 0.045 | 120 / 120 / 120 |
| Google Chrome | 1.809 / 2.700 | 0.196 / 0.300 | 0.025 / 0.100 | 120 / 120 / 120 |
| Brave | 2.132 / 2.800 | 0.239 / 0.300 | 0.037 / 0.100 | 120 / 120 / 120 |

Provenance: existing user-entered app record; measurement timestamp and
moving/idle conditions were not recorded.

### Slice 5A: WAITCNT / prefetch

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples |
| --- | --- | --- | --- | --- |
| Native Linux | Not measured | Not measured | Not measured | — / — / — |
| Google Chrome | Not measured | Not measured | Not measured | — / — / — |
| Brave | Not measured | Not measured | Not measured | — / — / — |

No host benchmark was recorded for this slice.

### Slice 6: ARM / Thumb counter

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples |
| --- | --- | --- | --- | --- |
| Native Linux | 2.099 / 3.338 | 0.099 / 0.155 | 0.023 / 0.036 | 120 / 119 / 120 |
| Google Chrome | 3.390 / 4.200 | 0.248 / 0.400 | 0.036 / 0.100 | 120 / 120 / 120 |
| Brave | 3.421 / 4.000 | 0.258 / 0.400 | 0.027 / 0.100 | 120 / 120 / 120 |

Attribution unresolved: these values were recorded under the counter heading,
which incorrectly repeated the copy-fixture description. They are preserved as
that heading’s timing record; the measured ROM was not independently identified.
Native conversion has 119 samples, while core/submission have 120.

### Slice 7: CPU diagnostics

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples |
| --- | --- | --- | --- | --- |
| Native Linux | 2.811 / 3.575 | 0.149 / 0.204 | 0.032 / 0.047 | 120 / 120 / 120 |
| Google Chrome | 4.252 / 5.100 | 0.273 / 0.400 | 0.043 / 0.100 | 120 / 120 / 120 |
| Brave | 4.207 / 4.700 | 0.253 / 0.300 | 0.045 / 0.100 | 120 / 120 / 120 |

The measured diagnostic ROM was not identified; these app values are not
assigned to both ARM and Thumb. They describe steady result-screen callbacks.
Measurement timestamps were not recorded. Baseline: `ec4de485b28b2d1357adf954469e162ba793700e`
plus Slice 7 changes; record updated 2026-09-30.

### Slice 8: Tiled background

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples |
| --- | --- | --- | --- | --- |
| Native Linux | 4.179 / 5.523 | 0.120 / 0.152 | 0.028 / 0.036 | 120 / 120 / 120 |
| Google Chrome | 5.557 / 6.700 | 0.185 / 0.300 | 0.037 / 0.100 | 120 / 120 / 120 |
| Brave | 5.539 / 6.800 | 0.177 / 0.300 | 0.033 / 0.100 | 120 / 120 / 120 |

Record date: 2026-09-30; baseline `e9ca2d6` plus Slice 8 changes.
Native provenance: `native.txt` and user entries; browser provenance: user entries.
Moving/idle input conditions were not recorded.

### Slice 9: Sprite scene

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples |
| --- | --- | --- | --- | --- |
| Native Linux | 4.569 / 6.183 | 0.113 / 0.150 | 0.028 / 0.036 | 120 / 120 / 120 |
| Google Chrome | 6.213 / 7.400 | 0.183 / 0.300 | 0.028 / 0.100 | 120 / 120 / 120 |
| Brave | 3.961 / 7.700 | 0.170 / 0.300 | 0.023 / 0.100 | 120 / 120 / 120 |

Record date: 2026-09-30; baseline `e3b7d19` plus Slice 9 changes.
App values are recorded user entries; moving/idle classification was not recorded.
The Chrome conversion mean was written `0.183 m`; its unit is corrected to ms,
with the numeric value unchanged. Native capture provenance was not supplied.

## Headless benchmarks

| Record | Samples / conditions | Core mean (ms) | Core p95 (ms) | Unthrottled speed |
| --- | --- | --- | --- | --- |
| Historical Slice 8 | 600 complete emulated frame advances after setup and 19-frame replay; released input | 1.092 | 1.111 | Approximately 15.33×, previously recorded from rounded mean |
| Historical Slice 9 | 600-frame benchmark reported completed; elapsed timings absent | Not measured | Not measured | Not measured: elapsed timing absent |
| Historical Slices 0–7 | No complete-frame benchmark recorded | Not measured | Not measured | Not measured: no complete-frame record |

These are core-only complete emulated-frame timings; conversion, UI, texture,
and GPU costs are excluded. Each target advances by 280,896 cycles. Headless
conversion and texture submission are not applicable. Fresh closeout results
are appended separately with their own environment provenance.

## Guest diagnostic completion times

| Historical record | ARM (ms) | Thumb (ms) | Services (ms) | Samples / provenance |
| --- | --- | --- | --- | --- |
| Slice 7 user-entered run | 1.814 | 1.742 | 0.259 | One completion each, user-supplied fixture transcript |
| Slice 8 retained transcript | 0.940 | 0.955 | 0.130 | One completion each, retained CLI output |

`completion_ms` includes ROM loading and execution to the terminal instruction;
subsequent scanout and presentation are excluded. Historical memory diagnostic
completion time: Not measured (no millisecond entry retained).

### Earlier guest execution records

| Transcript provenance | Pixels execution (ms) | Calculations execution (ms) | Copy execution (ms) | Samples |
| --- | --- | --- | --- | --- |
| WAITCNT section transcript | 5.335 | 5.267 | 4.626 | One run each |
| Slice 7 user-supplied transcript | 9.748 | 9.916 | 8.779 | One run each |
| Slice 8 retained transcript | 5.615 | 5.653 | 4.979 | One run each |

These `execution_ms` records include machine loading, terminal execution,
memory/mailbox inspection and two additional emulated frame periods for scanout;
image comparison and presentation are excluded. They are not pure terminal
completion timings or single-frame benchmarks. No historical counter or
WAITCNT guest completion milliseconds were recorded.

## Profiling results

Optimized release builds with `CARGO_PROFILE_RELEASE_DEBUG=1`; Callgrind windows
include initialization and scripted replay plus 60 benchmark frames. Instrumented
wall times are excluded. Counts are host instruction references, not guest
cycles or milliseconds. No tile or sprite caches were added.

| Historical profile | Host instruction references | Provenance |
| --- | --- | --- |
| Slice 8 total | 1,877,851,931 | performance.md record; `/tmp/gba-tiled-results/scanline.callgrind` |
| Slice 8 total, separate retained record | 1,877,851,986 | tiled_background.md record; discrepancy unresolved, neither value replaced |
| Slice 9 total | 2,047,673,889 | `/tmp/gba-sprite-results/scanline.callgrind` |

| Source operation | Slice | Host instruction references | Recorded profile share |
| --- | --- | --- | --- |
| Map offset calculation | 8 | 23,750,400 | 1.26% |
| Map-entry read | 8 | 23,750,400 | 1.26% |
| Packed 4bpp nibble extraction | 8 | 11,875,200 | 0.63% |
| Palette color read | 8 | 31,172,760 | 1.66% |
| OBJ tile-number calculation | 9 | 163,010 | Not recorded |
| OBJ palette reads | 9 | 279,220 | Not recorded |
| OAM winner storage | 9 | 111,688 | Not recorded |
| OBJ/BG priority comparison | 9 | 55,844 | Not recorded |

Slice 8 scanline work is inlined into `System::advance_time`. Source reports
were recorded as `scanline-profile.txt` in the corresponding temporary evidence
directories; those paths are historical provenance, not durable artifacts.

## Performance target comparison

| Target | Recorded comparison |
| --- | --- |
| Core p95 <12 ms | All recorded app callback p95 values are below 12 ms; callbacks are not full frames |
| Complete-frame core p95 <12 ms | Historical Slice 8 headless p95 1.111 ms meets this core-only target |
| Unthrottled ≥1.25×, aim 1.5× | Historical Slice 8 headless record approximately 15.33× meets the target |
| Total frame work within 16.7427 ms | Not measured: no end-to-end app/GPU frame timing |
| Sustained 100% clock on Linux / Chrome / Brave | Not measured: no cycles/wall-time interval recorded |
| 30-minute resource stability | Not measured: no duration/resource series recorded |
| Audio queue / underruns | Not applicable to Slices 0–9; audio is outside this scope |
| Routine allocation profile | Not measured: no allocation-count profile recorded |

## Closeout CLI measurements 

Environment checked for this run: Intel Core i7-13620H (10 cores / 16 threads),
Arch Linux rolling, rustc 1.98.1, release opt-level 3 / thin LTO, tracing disabled.
AC adapter online; platform power profile **balanced**. These sequential local
CLI runs do not replace the historical AC/performance-profile measurements.
No concurrent validation build was scheduled during these final benchmark runs;
other host load and thermal state were not recorded. Node v26.10.0 and Python
3.14.7 were used for runtime verification; no new browser timing was collected.

| Scenario | Complete emulated frames | Core mean (ms) | Core p95 (ms) | Core p95 <12 ms |
| --- | --- | --- | --- | --- |
| pixels | 600 | 0.760 | 0.809 | Yes |
| tiled | 600 | 1.145 | 1.200 | Yes |
| sprites | 600 | 1.174 | 1.207 | Yes |

Sample boundaries: 280,896-cycle absolute advances, after initialization and
settled replay where applicable; conversion/submission/GPU work excluded.
Each benchmark rechecks the guest image. Raw logs: `final-bench-{scenario}.txt`
in `/tmp/gba-closeout` (temporary provenance). Reproduce with
`cargo run --locked -p gba-tools --release -- bench --scenario SCENARIO --frames 600`.

| Guest | Terminal completion (ms) | Samples |
| --- | --- | --- |
| arm | 0.977 | 1 |
| thumb | 0.987 | 1 |
| services | 0.126 | 1 |
| memory | 0.957 | 1 |

Completion boundaries match the historical diagnostic table: ROM loading through
terminal execution, excluding subsequent scanout and presentation. Source:
`/tmp/gba-closeout/fixtures.txt`, final sequential fixture run.

| Guest | Execution including additional scanout (ms) | Samples |
| --- | --- | --- |
| pixels | 5.806 | 1 |
| calculations | 5.739 | 1 |
| copy | 5.309 | 1 |

These use the earlier `execution_ms` boundary, not pure terminal completion.
No fresh sustained app-speed or GPU/presentation measurement was made. No new
unthrottled-speed value was recorded; the historical 15.33× figure remains
unchanged. The complete-frame core p95 results meet the 12 ms core-only target
under these local conditions; total application-frame cost remains unmeasured.

## Slice 10: VBlank IRQ / HALT

The default app now loads `irq-sprites.gba`: the existing scrolling background,
player, priority and object-layout scene waits in HALT instead of polling display
status. The original polling `sprites.gba` remains available. Both variants match
the same five scripted full-frame checkpoints. A separate `vblank.gba` guest
checks interrupt dispatch, acknowledgement, masked wake and disabled-source stall.
Detailed plan acceptance and manual commands: [VBlank interrupt record](vblank_interrupt.md).

### Application measurements

Each platform row is reserved for the IRQ sprite scene. Previous user-entered
millisecond measurements are preserved above; no app timing was inferred from
headless measurements.

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples | Visual acceptance |
| --- | --- | --- | --- | --- | --- |
| Native Linux | 0.942 / 1.223 | 0.123 / 0.157 | 0.024 / 0.036 | 120 / 120 / 120 | Not verified; manual check pending |
| Google Chrome | 1.442 / 1.800 | 0.241 / 0.300 | 0.037 / 0.100 | 120 / 120 / 120 | Pending user check |
| Brave | 1.478 / 2.000 | 0.242 / 0.300 | 0.032 / 0.100 | 120 / 120 / 120 | Pending user check |

### Automated execution and headless measurements

Baseline: `a41eec4cd267dd21f7484610e4721121e8e9ef79` plus Slice 10 changes.
CPU rechecked: Intel Core i7-13620H, 10 cores / 16 threads; rustc 1.98.1;
release opt-level 3 / thin LTO; platform power profile balanced. OS, AC state,
thermal state and other host load were not independently refreshed for this run.
Benchmarks ran sequentially after validation builds completed.

| Scenario | Complete emulated frame samples | Core mean (ms) | Core p95 (ms) | Core p95 <12 ms |
| --- | --- | --- | --- | --- |
| IRQ sprite scene, after 20-frame replay with keys released | 600 | 0.229 | 0.235 | Yes |
| VBlank backdrop diagnostic, after eight-frame verification | 600 | 0.143 | 0.146 | Yes |

Samples are absolute advances of 280,896 cycles. Loading, initialization and
scripted replay precede the measured window. Conversion, texture submission,
GPU and host pacing are excluded. The runner rechecks scene output after the
benchmark. Raw logs: `/tmp/gba-vblank-results/bench-irq-sprites.txt` and
`/tmp/gba-vblank-results/bench-vblank.txt` (temporary provenance).

| Guest evidence | Result |
| --- | --- |
| IRQ sprite initialization | 425,957 cycles |
| IRQ sprite replay | 5,617,920 cycles; 59,833 instructions; five 38,400-pixel comparisons passed |
| Original polling sprite replay, same input/checkpoints | 5,617,935 cycles; 491,556 instructions; five image comparisons passed |
| IRQ scene callback / sleep | IRQ count equals scene update count at each checkpoint; IF before acknowledgement = 1, after = 0; CPU halted |
| VBlank diagnostic | Eight frames; 2,247,168 cycles; 274 instructions; eight callbacks and wakes; completed image reflects preceding callback |
| Disabled VBlank source | Expected stall: eight frames elapsed, zero callbacks and wakes; CPU remains halted; WASM diagnostic executes 34 setup instructions |
| IME disabled / CPSR.I set | Eight wakes, zero callbacks; ordinary guest acknowledgement permits returning to HALT |
| Thumb interrupted / returned | Eight callbacks and wakes; Thumb state restored |
| Firmware not mapped | Exception fetch at 0x18 rejected before the guest callback |
| Fixture build | Original ROMs rebuilt or upstream identities verified; new ROM and firmware SHA-256 checked against manifest |
| Regression checks | All 17 manifest fixtures passed; workspace tests passed (8 core, 7 session); Clippy with warnings denied passed |
| WASM execution | 24 WAITCNT cases, ARM/Thumb/services/memory diagnostics, all VBlank diagnostic variants and unmapped-vector rejection passed in Node |
| Application builds | Native workspace test compilation, WASM app check and Trunk release build passed |
| Formatting | cargo fmt check and git diff whitespace check passed |

Firmware SHA-256: `044a1305b3e47fcd568e77fcd792f251cd2cb85b359949931654ebb399a37498`. Fixture identities, cycle/instruction budgets,
mailbox and source paths are frozen in `fixtures/manifest.toml`. The firmware
remains test-only and is mapped explicitly for recognized original fixtures.

The headless p95 meets the core-only target. Total app-frame time, sustained
Linux/Chrome/Brave cycles per wall second, allocation counts and 30-minute
resource stability are not measured. Browser visual/runtime acceptance remains
pending for the user; a WASM core run or build does not establish it. Audio is
outside Slice 10. Slice 10A verification is recorded in [keypad_interrupt.md](keypad_interrupt.md). Slice 11 DMA evidence is recorded below.


## Slice 11: DMA3 scene uploads

Record date: 2026-09-30. Baseline: `7e873b1457dee2c30993678af56a5436bb80f9ad`
plus the current Slice 11 changes. DMA3 now copies one bounded read/write beat
at a time through the existing memory timing and device synchronization paths.
The CPU yields bus ownership during transfers; IRQ requests continue to latch,
and exception delivery resumes after completion. Immediate, VBlank and visible
HBlank triggers, halfword/word width, zero-count expansion, increment/decrement/
fixed address control and destination reload are implemented. DMA0–2 and special
sound/capture triggers are outside this slice.

The original `dma.gba` guest uploads a nonzero 64 KiB pattern to VRAM, then clears
the 8 KiB screen map. The runner checks every halfword against those two expected
regions, preventing untouched zero-initialized RAM from passing as a successful
upload. VBlank callbacks then arm DMA3 for the next VBlank: holding logical A
(keyboard Z) selects green tile data; releasing it selects red. The next complete
scanout displays the uploaded data. Firmware entry/return and IF acknowledgement
use the same path as the existing IRQ scenes.

### Plan acceptance checklist

| Slice 11 requirement | Evidence / status |
| --- | --- |
| Button swaps tile/sprite data through DMA3 at the selected display event | Original tiled guest swaps tile 0 red/green at VBlank; app load/replay controls use the shared logical input timeline |
| Transfer width, count, address control, triggers and bus timing needed by demo | Guest exercises halfword immediate uploads and fixed-source word VBlank uploads; existing bus costs charge both source and destination; focused HBlank repeat test verifies word width, source progression, destination reload and disable |
| Devices advance during transfer; retain state between bounded beats | Large-upload regression yields with data partially copied and enable still set, retires no CPU instructions, resumes to completion and preserves the intervening VBlank IF request |
| Copied bytes and resulting picture match | Full 64 KiB setup comparison plus all 16 tile halfwords at each checkpoint; five full 38,400-pixel comparisons pass on native and compiled WASM |
| Event order | Frame 6/9 contains newly uploaded tile data while scanout still shows the previous color; frame 7/10 shows the new color; exact VBlank/completion counts and saved IF 0x0801 → 0 match |
| IRQ behavior remains working | Earlier VBlank dispatch/masked wake/disabled stall/Thumb-return/unmapped-vector and keypad negative/wake checks pass; IRQ sprite scene retains its five exact captures |
| App remains responsive during large transfers | Bounded core progress verified; Linux/Chrome/Brave visual responsiveness, pause/reset/focus-loss and live interaction remain pending user checks |

Two focused Rust regressions were added to the existing core test module.
Both failed on the pre-DMA core because destination data was not copied, then
passed with DMA implemented. They catch deadline monopolization/lost device
requests and repeat uploads incorrectly rewinding the source or failing to reload
the destination. No new test module or per-function test suite was added.

### Frozen guest execution evidence

ROM: `roms/dma.gba`, 380 bytes; SHA-256:
`f515710122c58b37e3487ae0ab51c93b89b94f56178270edb49a6969ac9ae6e7`.
Startup is cartridge-direct ARM with explicitly mapped original test firmware.
Firmware SHA-256 remains
`044a1305b3e47fcd568e77fcd792f251cd2cb85b359949931654ebb399a37498`.
Startup completion executes declared ready PC `0x080000A8` with mailbox ID
`0x0065`, after 221,638 cycles. Total replay bounds: 100,000 instructions and
4,000,000 cycles. Final replay: 2,808,960 cycles, 402 instructions, CPU halted.
The initial uploads finish after the first VBlank edge, so the IRQ counters begin
with the second frame; the frozen expectations account for that startup interval.

| Frame | Tile halfword | Complete-frame color | VBlank callbacks | DMA completion callbacks |
| --- | --- | --- | --- | --- |
| 4 | 0x1111 | 0x001F (red) | 3 | 2 |
| 6 | 0x2222 | 0x001F (red) | 5 | 4 |
| 7 | 0x2222 | 0x03E0 (green) | 6 | 5 |
| 9 | 0x1111 | 0x03E0 (green) | 8 | 7 |
| 10 | 0x1111 | 0x001F (red) | 9 | 8 |

At each checkpoint the shared callback saves IF `0x0801`, acknowledges it to
zero and returns to HALT. Native and compiled-WASM runs match these counts,
cycles, instructions, copied data and complete-frame colors. Input events and
checkpoints are frozen in `fixtures/manifest.toml`; the native runner checks
`roms/dma/input.rs` against the manifest before accepting the app replay timeline.

### Application measurements

Earlier user-entered millisecond values are preserved. These rows apply only to
the new DMA scene; no app measurements are inferred from CLI benchmarks.

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples | Visual acceptance |
| --- | --- | --- | --- | --- | --- |
| Native Linux | 0.838 / 1.174 | 0.126 / 0.166 | 0.025 / 0.036 | 120 / 120 / 120 | Pending user check |
| Google Chrome | 1.112 / 1.500 | 0.242 / 0.400 | 0.033 / 0.100 | 120 / 120 / 120 | Pending user check |
| Brave | 1.167 / 1.500 | 0.242 / 0.300 | 0.044 / 0.100 | 120 / 120 / 120 | Pending user check |

Provenance: user-supplied Slice 11 app measurements for Linux, Chrome and Brave.
Measurement timestamp and moving/idle input conditions were not recorded.
These callback timings do not establish full-frame cost or visual acceptance.

### Headless frame measurements

Environment refreshed: Intel Core i7-13620H, 10 cores / 16 threads; rustc 1.98.1;
platform power profile balanced. Release opt-level 3 / thin LTO; tracing disabled.
OS/desktop, AC state, thermal state and other host load were not independently
refreshed for this run. Benchmarks ran sequentially after validation builds
completed, with logical keys released.

| Scenario | Complete emulated frame samples | Core mean (ms) | Core p95 (ms) | Core p95 <12 ms |
| --- | --- | --- | --- | --- |
| DMA tile scene after 10-frame replay | 600 | 0.196 | 0.198 | Yes |
| Earlier IRQ sprite scene after 20-frame replay | 600 | 0.229 | 0.239 | Yes |

Each sample advances an absolute target by 280,896 cycles. Initialization,
including the large upload, and scripted replay precede the measured window.
Conversion, texture submission, GPU work and host pacing are excluded. The DMA
scene still performs one eight-word tile upload each VBlank during measurement;
its final image is rechecked after the run. These numbers establish the core-only
p95 target for these scenarios. Large-upload application callback timing,
end-to-end frame cost and sustained app speed remain not measured.
Sources: `/tmp/gba-dma-results/bench-dma.txt` and `bench-irq-sprites.txt`.

### Automated validation and reproduction

- All 20 manifest fixtures rebuilt or upstream identities verified, and executed successfully.
- Workspace tests: 17 passed (10 core, 7 session); app/tools and doc-test targets completed.
- Clippy with warnings denied, formatting and diff whitespace checks passed.
- Native app release build and Trunk 0.21.14 WASM release artifact build passed.
- Compiled-WASM Node runner: 24 WAITCNT cases, ARM/Thumb/services/memory diagnostics,
  VBlank variants, both keypad contracts and all five DMA checkpoints passed.

Temporary raw evidence: `/tmp/gba-dma-results/build-fixtures.txt`, `fixtures.txt`,
`wasm.txt`, `frame.dma-frame-{4,6,7,9,10}.ppm` and the benchmark logs. These paths
are local temporary provenance, not durable release artifacts. Reproduce:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-tools --release -- build-fixtures
cargo run --locked -p gba-tools --release -- fixtures run
python3 roms/cartridge/verify_wasm.py --diagnostics --vblank --keypad --dma
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo run --locked -p gba-tools --release -- bench --scenario dma --frames 600
cargo run --locked -p gba-tools --release -- bench --scenario irq-sprites --frames 600
```

### Manual Linux / browser checks

Linux:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

Browser, in a separate terminal if Linux is also running:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release
```

Open the printed local URL in Chrome and Brave. On each platform, click
**Load DMA tile scene**, hold **Z** to turn the tiled screen green, then release
it to return to red. Click **Replay DMA input**; it pauses at frame 10, with a red
frame and status reporting 9 VBlanks, 8 uploads and acknowledged IF. Repeat load/
reset during the large setup upload, and check pause/resume and focus loss remain
responsive. Recheck **Replay IRQ sprite input** and both keypad replays. Record
app timings with the existing benchmark controls and fill the platform rows above.

Full Slice 11 visual acceptance remains pending these manual checks. Total app
frame cost, sustained platform cycles/wall-time, allocation counts and 30-minute
resource stability are not measured. Audio remains outside Slice 11.

## Slice 12: Synchronized PCM scene

Record date: 2026-09-30. Baseline: `3e0eda490278a94dc67f6a5ff10a0311aa1915ca`
plus the current Slice 12 changes. The default application loads `pcm.gba`.
Holding logical A (keyboard **Z**) makes the square green and selects a 2048 Hz
tone; releasing it restores red and silence. One normal VBlank callback changes
both the palette entry and DMA1 source. The guest sleeps in HALT between callbacks.

The core now owns timer counters/reloads, prescalers, overflow IRQs, cascade
progression, a 32-byte Direct Sound A FIFO, four-word DMA1 refill bursts and
signed mono PCM at 32,768 Hz (one sample every 512 master cycles). FIFO mode
forces word width and a fixed destination, ignores the programmed count, and
retains source progression across refills. DMA beats still yield at bounded
host deadlines and synchronize devices through the existing bus path.
This slice uses the baseline A/timer-0 mono path. Sound-register mixer routing,
channel B, master-enable and bias/PWM semantics remain Slice 12A work.

Host adapters share an integer-phase streaming linear resampler. Linux CPAL
consumes a bounded lock-free queue; browser AudioWorklet processing consumes a
preallocated ring. Neither callback executes the machine, blocks on a lock,
logs or allocates. Browser chunks are copied into ordinary owned ArrayBuffers,
transferred and recycled; WASM linear memory is never detached. Queue capacity
is 80 ms with a 60 ms initial/recovery priming target. Overflow and underrun
frames are counted. Initial priming silence is not an underrun. Lifecycle
boundaries invalidate old samples, clear core staging and reset interpolation.
Mute and volume apply to host playback without changing guest hardware state.

### Plan acceptance checklist

| Slice 12 requirement | Evidence / status |
| --- | --- |
| Button changes a visible object and plays a sound | Original guest changes the 16×16 square at (112, 72) and DMA1 source in one VBlank callback; five full-image and exact PCM hash checkpoints pass on native and compiled WASM |
| Timer overflow, Direct Sound FIFO consumption and DMA1 refill | Focused regression checks timer IF, forced four-word FIFO bursts, fixed destination, source progression, signed levels and identical output across small/large deadlines; guest replay has zero empty-FIFO events |
| Core PCM follows emulated time | 5,486 samples after 2,808,960 cycles = floor(cycles / 512), at 32,768 Hz; no staging drops; native/WASM match |
| Streaming resampling and native CPAL output | Uneven chunks match continuous conversion at 44,100/48,000/96,000 Hz; actual native device probe streams at 48,000 Hz with no callback errors |
| Browser Worklet output and gesture startup | Chrome and Brave report running 48,000 Hz contexts and nonzero output frames with zero underruns/overflow; audible playback and repeatable gesture/unlock behavior remain unverified |
| Volume/mute | Shared frontend controls implemented; audible behavior remains pending live interaction checks |
| Repeated interaction, pause/resume and reset stay synchronized | Session regression rejects pre-boundary staging; resampler restart matches a fresh stream; eight-second native CLI probe exercises input, pause/resume, reset and focus suspension/resumption without underruns or overflow. Physical audiovisual alignment and native UI responsiveness were not measured; Chrome/Brave runtime checks remain pending |
| Measure queue depth, underruns and output rate | Native probe and user-supplied Linux/Chrome/Brave live counters recorded below; all captured queue maxima remain under 80 ms with zero underruns/overflow |

Three focused Rust tests were added. Each was observed RED before its missing
behavior was implemented, then GREEN: missing timer/FIFO/DMA sound progression,
stale staged PCM across lifecycle boundaries, and resampling phase/rate loss
across uneven chunks. No per-function or duplicate-layer test suite was added.
The original guest fixture supplies the required audiovisual integration evidence.

### Frozen guest execution evidence

ROM: `roms/pcm.gba`, 460 bytes; SHA-256:
`80dfae4f8694e6349c624969d1991677780be31eee22387c1b9cdb0538e2b5d7`.
Startup: cartridge-direct ARM, explicitly mapped original test firmware. Firmware
SHA-256: `044a1305b3e47fcd568e77fcd792f251cd2cb85b359949931654ebb399a37498`.
Declared ready instruction: `0x080000DC`; mailbox ID `0x0066` at `0x03000000`.
Ready reached after 952 cycles. Replay bounds: 100,000 instructions and
4,000,000 cycles. Final replay: 2,808,960 cycles, 365 instructions, CPU halted,
10 VBlank callbacks, two button transitions and acknowledged IF = 0.

| Frame | Square color | Button state | Transitions | PCM samples in that frame | Expected PCM |
| --- | --- | --- | --- | --- | --- |
| 4 | Red (0x001F) | Released | 0 | 549 | Silence |
| 6 | Green (0x03E0) | Held | 1 | 548 | Alternating signed +32 / -32, eight core samples per level |
| 7 | Green (0x03E0) | Held | 1 | 549 | Same waveform, continuous phase |
| 9 | Red (0x001F) | Released | 2 | 548 | Silence |
| 10 | Red (0x001F) | Released | 2 | 549 | Silence |

Each checkpoint compares all 38,400 pixels and the SHA-256 of actual signed-byte
PCM. Frozen hashes and input events are in `fixtures/manifest.toml`; the native
runner also checks the app's included input timeline against that manifest.
Silence hashes are derived from zero bytes. Stable active-window hashes are
computed from the analytic +/-32 waveform, with frozen startup phase
`(global sample index + 10) modulo 16`. The FIFO backlog introduces a bounded
transition tail; the checkpoints verify stable windows after that tail.
Core PCM timing does not establish physical audiovisual alignment: completed
scanout and host audio buffering have distinct presentation delays.

### Application measurements

All earlier user-entered millisecond timings are preserved. These rows are for
`pcm.gba` only. The following values were supplied by the user from the native,
Chrome and Brave application status displays on 2026-09-30. CLI core timings
are not substituted for application measurements.

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples | Visual/audio acceptance |
| --- | --- | --- | --- | --- | --- |
| Native Linux | 0.890 / 1.181 | 0.131 / 0.161 | 0.026 / 0.040 | 120 / 120 / 120 | Live streaming counters recorded; visual/audible alignment and controls not verified |
| Google Chrome | 1.115 / 1.500 | 0.239 / 0.300 | 0.035 / 0.100 | 120 / 120 / 120 | Running audio context and counters recorded; remaining live acceptance not verified |
| Brave | 1.171 / 1.800 | 0.262 / 0.400 | 0.048 / 0.100 | 120 / 120 / 120 | Running audio context and counters recorded; remaining live acceptance not verified |

Provenance: user-pasted application status for each platform. Exact timestamps,
measurement duration, browser versions and held/released input conditions were
not recorded. These are bounded core callback, conversion and CPU texture
submission timings; they do not measure complete application frames or GPU
presentation and do not establish sustained speed or physical synchronization.

### User-recorded live audio counters

| Field | Native Linux | Google Chrome | Brave |
| --- | --- | --- | --- |
| Displayed output rate | 8,000 Hz | 48,000 Hz (running) | 48,000 Hz (running) |
| Queue depth at capture | 74.6 ms | 52.6 ms | 61.4 ms |
| Maximum observed queue depth | 77.7 ms | 76.6 ms | 76.3 ms |
| Queue capacity | 80 ms | 80 ms | 80 ms |
| Underrun frames | 0 | 0 | 0 |
| Overflow frames | 0 | 0 | 0 |
| Output frames | 1,148,416 | 2,440,704 | 621,056 |
| Device callback errors | 0 | Not exposed by this browser status | Not exposed by this browser status |
| Core output cadence | 32,768 Hz | 32,768 Hz | 32,768 Hz |
| Core samples produced | 916,048 | 1,049,810 | 505,367 |
| Core staging drops | 0 | 0 | 0 |
| Empty-FIFO events | 0 | 0 | 0 |

Source: user-pasted live application status, recorded 2026-09-30. Native output
rate is preserved exactly as pasted: **8,000 Hz**, distinct from the earlier
48,000 Hz CLI probe below. The difference was not independently investigated.
All captured queue maxima are below the 80 ms cap, with zero reported underruns,
overflows, staging drops and empty-FIFO events. Counters can span different
lifecycle/reset intervals; their totals are not used to infer session duration,
sustained speed or wall-clock sample rate. Audible alignment, volume/mute,
pause/resume/reset behavior and a 30-minute stability session were not explicitly
confirmed by these captures and remain unverified.

### Native audio queue measurements

The production native adapter was measured by `gba-app --audio-probe`, which
opens no window and runs the actual guest/session/resampler/CPAL path. It runs
for approximately eight wall seconds, changes input, pauses for one second,
resumes, resets, suspends focus for one second, then resumes for two seconds.
No microphone/loopback recording or physical sound/image latency was measured.

| Field | Measured result |
| --- | --- |
| Negotiated native output rate | 48,000 Hz |
| Core output cadence | 32,768 Hz |
| Queue target / cap | 60 / 80 ms |
| Final queue depth | 58.0 ms |
| Maximum observed queued depth | 69.6 ms |
| Underrun frames after priming | 0 |
| Overflow frames | 0 |
| Output frames supplied to CPAL callbacks | 276,480 across the probe and lifecycle changes |
| Device callback errors | 0 |
| Post-reset core counters | 98,182 produced; 0 staging drops; 0 empty-FIFO events |
| Browser measurements | User-recorded Chrome/Brave live counters are in the separate table above |

Source: `/tmp/gba-pcm-results/audio-probe.txt`. The sample rate is the negotiated
stream rate, not an independently calibrated hardware clock. This short probe
establishes native streaming across discontinuities, not 30-minute stability or
physical synchronization. Reset/pause discard prepared historical audio; sound
already submitted to the OS/device cannot be recalled instantly.

### Headless frame measurements

Environment refreshed: Intel Core i7-13620H, 10 cores / 16 threads; rustc 1.98.1;
release opt-level 3 / thin LTO; tracing disabled. Power profile could not be
queried in the restricted environment and is not verified. OS/desktop, AC,
thermal state and other host load were not independently refreshed. Benchmarks
ran sequentially after release builds completed, with logical keys released.

| Scenario | Complete emulated frame samples | Core mean (ms) | Core p95 (ms) | Core p95 <12 ms |
| --- | --- | --- | --- | --- |
| PCM square after 10-frame replay | 600 | 0.193 | 0.196 | Yes |
| Earlier IRQ sprite scene after 20-frame replay | 600 | 0.292 | 0.297 | Yes |
| Earlier DMA tile scene after 10-frame replay | 600 | 0.235 | 0.243 | Yes |

Each sample advances an absolute deadline by 280,896 cycles. PCM production is
included; draining is outside the timed window. Host resampling/output, color
conversion, texture submission, GPU and host pacing are excluded. The earlier
IRQ/DMA core costs increase from the prior record because the core now also
produces clocked PCM, including silence. All three remain below the core-only
12 ms p95 target. Full application-frame cost, sustained platform speed,
allocation counts and 30-minute queue/resource trends remain not measured.
Sources: `/tmp/gba-pcm-results/bench-{pcm,irq-sprites,dma}.txt`.

### Automated validation and reproduction

- All 21 manifest fixtures rebuilt or upstream identities verified and executed successfully.
- Workspace tests: 20 passed (11 core, 9 session); app/tools and doc-test targets completed.
- Clippy with warnings denied, formatting and diff whitespace checks passed.
- Native app release build and Trunk WASM release artifact build passed.
- Compiled-WASM Node runner: 24 WAITCNT cases, ARM/Thumb/services/memory diagnostics,
  VBlank variants, both keypad contracts, DMA checkpoints and all five PCM checkpoints passed.
- Native device probe: zero underruns, overflow and device errors across the recorded lifecycle changes.
- Browser bridge and Worklet JavaScript syntax checked; user captures now record running Chrome/Brave output with zero underruns/overflow. Remaining audible and lifecycle acceptance is unverified.
- CI now installs the ALSA development dependency and runs the PCM WASM contract/JavaScript syntax checks; the hosted CI job itself was not executed locally.

Temporary raw evidence: `/tmp/gba-pcm-results/{tests,fixtures,build-fixtures,wasm,
native-build,trunk,audio-probe}.txt`, benchmark logs, and signed PCM/image captures
`frame.pcm-frame-{4,6,7,9,10}.{s8,ppm}`. These are local temporary provenance,
not durable release artifacts. Reproduce:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-tools --release -- build-fixtures
cargo run --locked -p gba-tools --release -- fixtures run
python3 roms/cartridge/verify_wasm.py --diagnostics --vblank --keypad --dma --pcm
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo run --locked -p gba-app --release -- --audio-probe
cargo run --locked -p gba-tools --release -- bench --scenario pcm --frames 600
```

### Manual Linux / Chrome / Brave acceptance

Linux interactive checks (visual/audible alignment remains unverified):

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

Browser:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release
```

Open the printed localhost URL in Chrome, then Brave. For each platform:

1. Click **Load PCM scene**, then **Enable audio**. Check the displayed device
   rate and queue counters. Expect a red square and silence initially.
2. Hold **Z**: the square turns green and a steady tone plays. Release **Z**:
   red and silence return. Repeat at least ten times and check consistent alignment.
3. Exercise **Volume** and **Mute** during the tone. Muting must leave the guest
   square and emulation running; unmuting must resume current sound.
4. During a held tone, **Pause**, then **Resume**. Pause releases Z and stops
   queued playback. Press Z again after resume. **Reset** must restore red/silence
   without replaying old samples; press Z again to check sound still works.
5. Switch focus and hide the tab during a tone, then return. Buttons must be
   released and old queued sound discarded. If the browser suspends its context,
   click **Enable audio** again and check that the displayed state becomes running.
6. Run **Replay PCM input**. It finishes paused with a red square, silence and
   two transitions. Run it again after reset. Recheck the earlier IRQ/DMA replays.
7. Record the actual output rate, queue range/max, underruns, overflows and app
   timing sample counts. Queue capacity must remain 80 ms; recurring underruns or
   overflow fail streaming acceptance. A 30-minute queue/resource session remains
   unmeasured until performed and recorded.

Slice 12 implementation and automated evidence are present. Full acceptance
remains open for physical audiovisual alignment, audible controls and the
Linux/Chrome/Brave live interaction checks; browser checks are owned by the user.


## Slice 12A: Sound-register mixer controls

Recorded 2026-09-30 against `2da964e68ab209f97bfc061b500049f257e4a3bd`
plus the current Slice 12A changes. Earlier Slice 12 measurements above remain
historical; user-entered timings were preserved. The default PCM scene now
initializes the sound registers explicitly and supports interactive mixer controls.

### Plan comparison

| Slice 12A requirement | Evidence / status |
| --- | --- |
| SOUNDCNT_H A/B routing, timer selection, volume, FIFO reset | Independent 32-byte FIFOs, timer 0/1 selection, half/full gain, stereo routing and self-clearing reset strobes implemented. Focused core regression checks B on timer 1 independently of A on timer 0; existing DMA regression now checks both FIFO destinations at all four PWM settings across 37/4096-cycle advance chunks. |
| SOUNDCNT_X master enable and readable status | Only bit 7 is writable; PSG activity bits are zero while PSG channels are absent. Master disable clears PSG control storage and silences PCM while retaining FIFO/timer state. Re-enable capture retains the original waveform phase. |
| SOUNDBIAS and output PWM behavior | Writable mask `0xc3fe`, default `0x0200`, unsigned 10-bit saturation, 9/8/7/6-bit quantization and 32768/65536/131072/262144 Hz PWM edges implemented. Higher-rate edges are averaged into bounded 32768 Hz stereo frames. Tests check quantization/clipping; the guest checks raised bias and 6-bit PWM. |
| SOUNDCNT_L storage/register semantics | Writable mask `0xff77`, writes ignored with master disabled, cleared by master disable, stored for later PSG integration. No PSG channel synthesis is claimed in this slice. |
| Extend the original PCM fixture and record output | Same `pcm.gba` scene, mailbox, IRQ/HALT path and original five PCM/image checkpoints retained. Nine additional stereo captures check routing, disable/resume, half volume, bias/PWM, FIFO reset and recovery. Native and compiled WASM agree on every frozen hash. |
| Existing synchronized PCM slice remains green | All five original PCM hashes are unchanged; workspace tests, complete fixture suite and prior compiled-WASM contracts pass. |
| Deterministic PCM and audible routing on Linux/browser | Deterministic native/WASM contracts pass. Production Linux device probe recorded below. Physical listening, interactive Linux acceptance and Chrome/Brave output acceptance remain not verified; run the manual checklist below. |

The register masks, mixer range and PWM settings follow
[GBATEK sound control documentation](https://rust-console.github.io/gbatek-gbaonly/#gba-sound-control-registers).
The PCM path models quantized DAC levels; it does not synthesize the MHz PWM
carrier or model speaker/capacitor filtering. Stereo frames remain intact through
host resampling, CPAL and the Worklet. Mono CPAL devices receive a downmix.
Core staging remains bounded to 4096 frames and host queue capacity remains 80 ms.

### Fixture identity and deterministic evidence

- Original MIT guest, cartridge-direct ARM startup, bundled test firmware required.
- ROM size: 568 bytes; SHA-256: `89b13fc1aa5f4d397324f7ded22f5830fc8cb3bf305b01bab8e276bcde78e5cb`.
- Ready/HALT loop: `0x080000f4`; setup: 997 cycles; mailbox `0x03000000 = 0x0066`.
- Bounds: 100,000 instructions / 9,000,000 cycles. Mixer replay ends after 28 frames at 7,865,088 cycles / 1,399 instructions, with 15,361 produced PCM frames and zero staging drops.
- Exactly two empty-FIFO pops occur after the two deliberate reset strobes. They are required by the fixture contract and are distinct from host device underruns. Frame 26 contains four zero PCM frames; frame 28 recovers the alternating eight-sample waveform runs.
- Captures encode signed DAC units (`PCM * 512`) as interleaved left/right little-endian i16. They are verification data, not conventional full-scale audio files. Before reset, zero-based sample phase is +11 modulo 16; recovery phase is +7. At 6-bit PWM, mixed transition samples average to +64 DAC units.

| Frame | Setting | Stereo frames | SHA-256 (native = WASM) |
| --- | --- | --- | --- |
| 12 | Stereo A | 549 | `ab1bdbaa250b0b0c29cab6ee05dd485bbb32ace3d2f5c55805b4e0ec03984ed3` |
| 14 | Left only | 548 | `1d17b23de2b859038d0d24e3dd859b6a0afd792bc2f73afd4d45f772fae0962c` |
| 16 | Right only | 549 | `71fe9a2898b6a75976b5145c66dcd3b328c7f589b6f1104d32f2cc3319b64ac2` |
| 18 | Master disabled | 549 | `47585db1c41fc8970c838bf60827198974f8bed7ae1958e222b6debe66d5e0c1` |
| 20 | Master resumed | 549 | `2e971d144fc028d3666c10feb864519037b0fc70f63e2d87709985f0712bcc17` |
| 22 | Half volume | 548 | `4f105a8df78053e4e1f3aeee86b394a7a6e960a830f4196f4cac02569e975522` |
| 24 | Raised bias / 6-bit PWM | 549 | `7b69d7c4a448056bcb524d0f5d97ced61765138c94d568d23577652f52925ae4` |
| 26 | FIFO reset | 549 | `11db43986e80ab2e5852966aecc7c995227c258bca10c142fd9d8a8f1a83ca4a` |
| 28 | FIFO recovery | 549 | `216050410791b4138eb80335b20c2903c4eae779253bed66b99f1b216ad904fa` |

### Validation and performance

One focused regression was added: master-disabled playback previously emitted
`0.25` instead of zero (RED); the implemented mixer silences output while FIFO
consumption continues (GREEN). The same regression checks timer selection, B
routing, gain, reset strobes, writable masks, status and DAC quantization/clipping.
The existing DMA regression was extended for B and PWM chunk independence.

- Workspace tests: 21 passed (12 core, 9 session); app/tools and doc tests completed.
- All 21 fixture identities rebuilt/verified and the full native fixture suite passed.
- Compiled WASM: original five PCM checkpoints and nine stereo checkpoints passed; the existing WAITCNT, ARM/Thumb/services/memory, VBlank, keypad and DMA contracts also passed.
- Clippy with warnings denied, native release build, WASM app check, Trunk release build, JavaScript syntax and formatting/whitespace checks: passed.

Headless complete-frame benchmark: 600 samples, mean **0.236 ms**, p95
**0.245 ms**, below the 12 ms core-only target. It runs after the 28-frame replay
with A held and default mixer settings. The historical Slice 12 result ran after
10 frames with A released; differing workload and concurrent build activity mean
these numbers are not a controlled regression comparison. Higher-PWM-mode frame
cost, full app-frame cost and physical audiovisual latency remain not measured.
Host resampling, playback, GPU work and PCM draining are outside the timed window.
Environment: x86_64 Linux, Intel Core i7-13620H, Rust 1.98.1, Node 26.10.0,
Trunk 0.21.14. Governor, thermals and exact audio device identity were not recorded.

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | App timing sample counts | Live mixer acceptance |
| --- | --- | --- | --- | --- | --- |
| Native Linux | 0.875 / 1.223 | 0.123 / 0.162 | 0.026 / 0.033 | 120 / 120 / 120 | Live counters recorded; zero device underruns/overflow/errors; listening/interactive acceptance not verified |
| Google Chrome | 1.152 / 1.500 | 0.243 / 0.300 | 0.031 / 0.100 | 120 / 120 / 120 | Running audio context; 115 underrun frames and 209 overflow frames recorded; streaming acceptance remains open |
| Brave | 1.123 / 1.600 | 0.246 / 0.400 | 0.039 / 0.100 | 120 / 120 / 120 | Running audio context; zero device underruns/overflow; listening/interactive acceptance not verified |

### User-recorded live mixer counters

Source: user-supplied Linux, Chrome and Brave application readouts for Slice 12A.
Each timing stage contains 120 samples. These are app callback timings, not
complete emulated-frame measurements. Browser versions, capture duration,
control sequence and whether counters were cleared before capture were not
recorded; output frame totals do not establish uninterrupted session duration.

| Platform | Output rate / state | Queue snapshot / maximum / capacity (ms) | Device underrun frames | Overflow frames | Output frames | Device errors |
| --- | --- | --- | --- | --- | --- | --- |
| Native Linux | 48,000 Hz | 69.9 / 75.8 / 80 | 0 | 0 | 770,048 | 0 |
| Google Chrome | 48,000 Hz / running | 62.6 / 80.0 / 80 | 115 | 209 | 2,883,341 | Not reported by browser adapter |
| Brave | 48,000 Hz / running | 54.7 / 77.7 / 80 | 0 | 0 | 1,170,432 | Not reported by browser adapter |

| Platform | Core PCM rate (Hz) | Produced frames | Staging drops | Empty-FIFO pops |
| --- | --- | --- | --- | --- |
| Native Linux | 32,768 | 472,401 | 0 | 132 |
| Google Chrome | 32,768 | 2,136,802 | 0 | 138 |
| Brave | 32,768 | 739,078 | 0 | 294 |

All recorded queue snapshots lie within the 40–80 ms buffering target, and all
recorded core callback p95 values are below 12 ms. Chrome reached queue capacity
and recorded both starvation and overflow; these counters prevent claiming clean
streaming acceptance for that capture. Their timing and recurrence are not known
from a single readout. Native and Brave recorded zero device underruns/overflow.
Core empty-FIFO pops are separate from host underruns: FIFO reset can produce
them deliberately, but the supplied control sequence does not establish the
cause of these particular counts. Audible routing, synchronization, lifecycle
behavior and a 30-minute queue/resource check remain not verified.

### Automated Linux device probe

The final eight-second Linux probe exercised left/right routing, pause/resume,
reset, focus suspension/resumption, master-disable, bias/PWM, half gain and FIFO
reset through the original guest and production stereo playback path:

| Native device metric | Recorded result |
| --- | --- |
| Negotiated output rate | 48,000 Hz |
| Active phase snapshots / maximum queue | 56.6–60.1 ms / 67.6 ms; resume 58.6 ms; capacity 80 ms |
| Device underrun / overflow / errors | 0 / 0 / 0 |
| Played device frames | 276,480 |
| Core counters at end (since reset) | 98,265 produced / 0 staging drops / 24 intentional empty-FIFO pops during held reset |

The probe establishes successful device streaming and counters, not physical
listening or audiovisual alignment. Its wall-clock scheduling makes final counts
host-dependent; the separate bounded fixture provides exact deterministic proof.
Browser output rate and queue/error snapshots for Slice 12A are recorded above;
long-session trends remain not measured. The previous Slice 12 browser counters
are historical.

Temporary evidence: `/tmp/gba-mixer-results/fixtures.txt`, `wasm.txt`, `trunk.txt`,
`audio-probe.txt`, and `pcm.mixer-frame-{frame}.s16le`; these are local temporary
artifacts, not durable release evidence. Reproduce automated output with:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-tools -- build-fixtures
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo run --locked -p gba-tools --release -- fixtures run --fixture pcm --capture /tmp/mixer
python3 roms/cartridge/verify_wasm.py --pcm
cargo run --locked -p gba-app --release -- --audio-probe
cargo run --locked -p gba-tools --release -- bench --scenario pcm --frames 600
```

### Manual Linux / Chrome / Brave acceptance

Linux:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

Browser (open the printed localhost URL in Chrome, then Brave):

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release
```

1. Click **Load PCM scene**, then **Enable audio**. Hold **Z**: expect a green square and a tone in both speakers. Stereo headphones make routing easier to verify.
2. While holding Z, hold **Left**, then **Right** separately: expect only the corresponding speaker. Release the arrow: expect both speakers again.
3. Hold **Down**: expect silence while the square stays green. Release Down: sound resumes; repeat ten times. The original tone cadence must continue, and device underruns/overflows must not grow.
4. Hold **Backspace**: expect half Direct Sound amplitude; release it to restore full gain. Hold **Up** to select the raised bias / 6-bit PWM setting; release it to restore the default. Core captures establish exact bias/quantization effects; speaker filtering may affect their audible character.
5. Tap **X** while holding Z: the FIFO clears and refills. Repeated or held X deliberately increments the core empty-FIFO counter; this must not be confused with host device underruns. Release X and check the tone recovers.
6. Repeat pause/resume, reset, mute/volume, focus loss and hidden-tab checks from Slice 12. **Replay PCM input** still runs the original ten-frame synchronized scene; the extra mixer chronology is exercised by the CLI/WASM contract and manual controls.
7. Record actual output rate, queue range/max, device underruns, overflow, timing sample counts and your millisecond timings above. Audible routing, physical synchronization and a 30-minute queue/resource check remain pending until recorded.

All five implementation requirements and automated contracts pass. Full Slice 12A
acceptance remains open for the manual Linux and browser listening/runtime checks.

## Slice 12B — cartridge backup identification (2026-09-30)

One core detector now selects cartridge configuration at ROM load, before guest
execution or persistence. Both Session and the native/WASM app use this path.
Detection scans word-aligned SDK library identifiers with three numeric version
characters (or the documented `nnn` placeholder). It recognizes `SRAM_V`,
`SRAM_F_V`, `EEPROM_V`, `FLASH_V`, `FLASH512_V` and `FLASH1M_V`.
EEPROM capacity remains unresolved until its serial protocol is implemented.
Unknown means unidentified hardware, not a claim that the cartridge has no save.
Different families produce `Ambiguous` with stable ordering and no guessed winner;
aliases of one family agree. Game titles never participate in selection.

The typed override accepts only None/Sram/Eeprom/Flash64/Flash128. A strict text
parser rejects invalid names. ROM evidence stays visible alongside the effective
selection. Overrides survive reset, while ordinary loading selects afresh. Failed
ROM validation retains the previous cartridge and selection. The app's **Backup
override for next ROM load** control applies when loading/reloading a cartridge,
not immediately when the menu changes. Hardware protocols and persistence are
outside this slice; slices 13–16 can consume `Machine::backup_selection()`.

### Frozen load-time fixtures

Original generated cartridges, reproducible with `roms/backup/build.py`, live in
`roms/backup/`. Their individual SHA-256 identities are frozen in
`roms/backup/manifest.json` and checked by the WASM verifier. Startup is direct
ARM with an entry self-branch; this is a loading contract, so no terminal mailbox,
CPU execution, framebuffer output or persistence is used as acceptance evidence.
Selection occurs with zero executed instructions and zero guest cycles.

| Cartridge | Automatic result |
| --- | --- |
| sram.gba / sram-fast.gba | Sram |
| eeprom.gba / override.gba | Eeprom |
| flash64.gba / flash64-old.gba / same-family.gba | Flash64 |
| flash128.gba | Flash128 |
| unknown.gba / malformed.gba | Unknown; no selected hardware |
| conflicting.gba | Ambiguous [Sram, Flash128]; no selected hardware |

The shared native/WASM contract checks automatic results, forces Flash64 for the
unknown, conflicting and EEPROM override cartridges, retains selection through
reset and rejected empty loads, clears overrides on a subsequent automatic load,
and rejects invalid text overrides. One focused regression was added: RED failed
on the SRAM fixture (`None` instead of `Some(Sram)`), then GREEN passed after
implementing detection. This catches missing/wrong selection at load, not an
internal protocol call sequence.

### Automated evidence and plan audit

Linux CLI validation on the current checkout:

- Workspace tests: 22 passed (12 core, one backup regression, nine session).
- Strict workspace Clippy, all targets: passed.
- Existing complete release fixture suite, including synchronized PCM/mixer: passed.
- Backup fixture rebuild: all 11 frozen hashes matched.
- Production core WASM execution: shared backup contract passed in Node.
- Native app compilation through workspace checks: passed.
- WASM app check and Trunk release build: passed.
- Formatting and diff whitespace checks: passed.

All six plan items are implemented: centralized load selection; four hardware
families and aliases; separation from protocols; validated overrides; recognized,
unknown, conflicting and forced fixtures with diagnostics; no title patches.
Native/WASM deterministic load behavior is verified by executable contracts.
User-provided native, Chrome and Brave readouts now verify the individual
load/detection cases recorded below and provide 120-sample runtime timings.
ROM-load latency remains **not measured**; these execution/conversion/submission
timings do not measure loading. Full manual acceptance remains pending for the
unrecorded fixture and override/lifecycle checks.

Reproduce automated evidence:

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 roms/backup/build.py
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo run --locked -p gba-tools --release -- fixtures run
python3 roms/backup/verify.py
cargo check --locked -p gba-app --target wasm32-unknown-unknown
env -u NO_COLOR trunk --config web/Trunk.toml build --release
```

### Manual native / Chrome / Brave checks

Native command (remaining UI checks pending):

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

Browser command; open the printed URL in Chrome and then Brave:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release
```

1. Set the override to **Auto detect**. Load each `roms/backup/*.gba` using the
   existing file picker/drop route. Compare the Backup diagnostic to the table;
   these cartridges intentionally display no scene.
2. For unknown and conflicting cartridges, confirm the explicit unresolved
   message. Select **Flash64** and reload the same file: selected hardware must be
   Flash64 while the original Unknown/Ambiguous evidence remains visible.
3. Force **Flash64**, load `override.gba`, and confirm Eeprom detection plus a
   Flash64 override. Reset: the selection and evidence must remain unchanged.
4. Select **Auto detect**, reload `override.gba`, and confirm Eeprom with no
   override. Force **None** and reload to confirm explicit no-save hardware.
5. Record native/Chrome/Brave results and any manually measured load timings.

### ROM picker follow-up (2026-09-30)

The shared app now has a **Load ROM** button at the start of the toolbar. It uses
pinned rfd 0.17.2 asynchronous dialogs, as specified by the plan's file-selection
stack. Native uses rfd's default XDG Portal/Zenity route and a worker thread;
WASM uses its local asynchronous executor. File bytes enter the existing loader,
including backup detection, the chosen override and controlled diagnostic firmware
selection. Drag-and-drop remains available. Cancelling does not replace the
cartridge, read errors are reported, and request generations discard stale results
when another scene or file is loaded. Opening the picker disables its button until
completion or a superseding load.

Workspace tests: all 22 passed. Native/WASM app checks and strict all-target
workspace Clippy passed. Native and browser release builds passed. Formatting and
diff whitespace checks passed. No new tests were added for the dialog UI;
Individual successful loads are recorded below. The readouts do not identify
whether picker or drag/drop was used. Supported Linux portal and Zenity fallback
behavior, picker cancellation, drag/drop and replacement during outstanding reads
remain manual checks. Picker latency is not measured; existing user timings are
preserved.

Use the native/browser commands above. Click **Load ROM**, choose
`roms/backup/sram.gba` (confirm the browser dialog if prompted), and expect
`Loaded sram.gba` with `Some(Sram)` / `Identified(Sram)` diagnostics. Repeat the
backup table and override/reset checks using the button. Also cancel a picker and
confirm the old cartridge remains loaded and the button becomes available again.
Confirm `pixels.gba` displays its bands through both picker and drag/drop.


### User-recorded native / Chrome / Brave results (2026-09-30)

The user reports correct ROM identification and black output for the backup
fixtures. Black output is expected: these cartridges loop without drawing a scene.
The following pasted readouts establish these specific automatic-load cases;
manual coverage of every other fixture and override/reset behavior is not recorded.

| Target | Loaded cartridge | Selected hardware | Detection | Override | Result |
| --- | --- | --- | --- | --- | --- |
| Native | eeprom.gba | Some(Eeprom) | Identified(Eeprom) | None | Expected identification |
| Chrome | sram.gba | Some(Sram) | Identified(Sram) | None | Expected identification |
| Brave | malformed.gba | None | Unknown | None | Expected unresolved result; explicit message displayed |

| Runtime metric | Native: eeprom.gba | Chrome: sram.gba | Brave: malformed.gba |
| --- | --- | --- | --- |
| Core execution mean / p95 | 3.689 / 5.246 ms | 5.635 / 6.900 ms | 5.503 / 6.700 ms |
| Pixel conversion mean / p95 | 0.093 / 0.131 ms | 0.183 / 0.300 ms | 0.162 / 0.300 ms |
| Texture submission mean / p95 | 0.023 / 0.034 ms | 0.030 / 0.100 ms | 0.033 / 0.100 ms |
| Samples per timing metric | 120 | 120 | 120 |
| Output rate / browser state | 48,000 Hz | 48,000 Hz / running | 48,000 Hz / running |
| Audio queue current / maximum / capacity | 65.9 / 80.0 / 80 ms | 68.8 / 80.0 / 80 ms | 48.0 / 66.3 / 80 ms |
| Device underrun frames | 0 | 0 | 0 |
| Device overflow frames | 250 | 306 | 0 |
| Device output frames | 1,909,760 | 332,800 | 813,568 |
| Device errors | 0 | not recorded | not recorded |
| Core PCM rate | 32,768 Hz | 32,768 Hz | 32,768 Hz |
| Core PCM produced | 926,395 | 437,760 | 616,749 |
| Core staging drops / empty FIFO | 0 / 0 | 0 / 0 | 0 / 0 |

These are runtime snapshots from different cartridges, not ROM-load benchmarks
or controlled cross-platform comparisons. Capture duration, browser versions,
host/device details and counter reset history were not recorded. Native and Chrome
show nonzero host overflow; its cause and recurrence cannot be determined from
these readouts. This does not change the observed backup-selection results.
The fixtures do not exercise save protocols, persistence or audible PCM behavior.

## Slice 13 — SRAM score recovery (2026-09-30)

Implemented only the SRAM score slice. Cartridge selection still comes from the
central Slice 12B detector/override. The core now owns a 32 KiB, initially `0xff`
SRAM chip mirrored through `0x0e000000..0x10000000`, with an eight-bit bus and
revision-tagged snapshots. CPU reset retains both bytes and dirty state. Guest
stores advance the revision only when a byte changes; imports always create a
newer revision. A completed older write cannot acknowledge a newer import.

The shared app loads SRAM before permitting guest execution. Saves use SHA-256
of the original ROM bytes, independent of filenames/titles. Native storage uses
`GBA_SAVE_DIR`, then `$XDG_DATA_HOME/gba-rs/saves`, then
`$HOME/.local/share/gba-rs/saves`. Browser storage uses the `saves` object store
in IndexedDB database `gba-rs-saves`, scoped to the page's origin.

One storage operation is in flight at a time. The latest dirty machine image is
captured for the next write, coalescing changes made during the previous write.
Completions carry identity, session generation and revision. Native success
requires fsync, verified staging bytes, rename, parent-directory sync and final
readback. IndexedDB success is reported only after transaction completion,
following [the transaction completion contract](https://developer.mozilla.org/en-US/docs/Web/API/IDBTransaction/complete_event).

Pause continues flushing dirty data. ROM replacement retains the old machine
until its save barrier completes. Ordinary native window close also waits for
storage; a failed write keeps the window open. Failed data remains dirty and
available through Retry/Export. Imports are disabled while storage is busy,
validate identity/size, create a newer dirty revision and restart the guest
paused so old CPU registers cannot overwrite the imported score. Export does
not acknowledge an autosave revision.

Portable `.gbasav` files contain `GBASRAM1`, the 32-byte ROM digest and exactly
32768 SRAM bytes (32808 bytes total). Raw/unidentified saves and exports for a
different ROM are rejected. Browser export requests a download; JavaScript
cannot acknowledge that the user has finished saving the downloaded file.
Browser shutdown cannot guarantee an asynchronous save completes: pause and
wait for **Saved revision** before closing, and retain an export.

### Fixture and automated evidence

The original MIT fixture is `roms/sram/score.s`, assembled using
`python3 roms/sram/build.py`. Its frozen identity and bounds live in
`roms/sram/manifest.json`; this separate fixture suite runs with the native
save probe and `python3 roms/sram/verify.py`. It uses direct ARM startup without
BIOS/test firmware. **Load SRAM score** uses these exact bytes through the normal
app loading/storage route. Tap/release **Z (A)** to add one point, up to sixteen.
Each point fills an eight-by-eight green cell at the top of the guest screen.

| Evidence | Result |
| --- | --- |
| ROM SHA-256 | `34ed9eb504ed6aa4da12d2fd29f408665c585afebb4f2dd4bc685f4bec523dfc` |
| Completion mailbox | `0x03000000 = 0x0073`; score at `0x03000002 = 1` |
| Bounded run | Three frames, 200000 instructions per frame, at most 842750 cycles |
| New chip, A held | Checkpoint instruction `0x08000118`; 842693 cycles; 78840 instructions; revision 3, dirty |
| Restored score, A released | Checkpoint instruction `0x08000110`; 842690 cycles; 78836 instructions; revision 1, clean |
| Exact scanout | All 38400 pixels checked: first eight columns of first eight rows `0x03e0`, remaining pixels zero |
| Score bytes | SRAM begins `a5 01`; remaining bytes `ff` |
| Portable image SHA-256 | `82683ad62321b068132e24d97e0fe4efdecf3ac2501a045918ff13a21beda432` |
| Native full-process reopen | PASS: separate write/read processes using the same production disk/envelope functions |
| Native export/import format | PASS: checked export replacement, readback/decode and fresh guest recovery; wrong identity rejected |
| Failed native replacement | PASS: non-directory destination rejected; probe does not claim UI failure-path acceptance |
| Revision/image contract | PASS on native/WASM: old acknowledgement leaves newer import dirty; short import rejected without changing bytes |
| Core RED → GREEN regression | One added test: previously `UnmappedAddress` at SRAM read; now erased-byte read, mirrored store and reset retention pass |
| Workspace tests | PASS: 23 (13 core unit, one existing backup contract, nine session) |
| Clippy / format | PASS: workspace/all targets with warnings denied; `cargo fmt --all -- --check` |
| Native/WASM app compilation | PASS |
| Existing release fixture runner | PASS: all existing fixture cases/checkpoints |
| Production WASM execution | PASS: shared session/guest contract in Node; does not exercise browser IndexedDB |
| Release Trunk build | PASS with Trunk 0.21.14 |
| Native app window/pickers/close barrier | Not verified visually; user-performed acceptance pending |
| Chrome / Brave IndexedDB, reopen, download/import, failure UI | PASS: user manually confirmed all browser checks, including close/reopen, import/export and failure paths |

Retained evidence: [native write](verification/sram-native-write.txt),
[separate-process reopen](verification/sram-native-reopen.txt), and
[production WASM contract](verification/sram-wasm.txt). Native probe data is in
`/tmp/gba-sram-final-20260930`; temporary files are not a permanent evidence store.
Live environment: Linux `7.2.7-arch1-1`, rustc `1.98.1`, cargo `1.98.1`.
Browser versions, refresh rate and zoom were not refreshed for this slice.

### Slice 13 timing fields

Existing millisecond entries above are unchanged. Chrome and Brave measurements
below are user-provided readouts for `sram.gba`, with 120 samples per operation.
Core p95 is below the Section 6 target of 12 ms on both browsers (8.100 ms and
8.400 ms). These callback measurements do not establish complete-frame/GPU
presentation timing or sustained emulation speed. Native UI timing, storage
latency and 30-minute resource trends remain not measured.

| Platform | Core mean / p95 (ms) | Conversion mean / p95 (ms) | Texture submission mean / p95 (ms) | Samples (core / conversion / submission) |
| --- | --- | --- | --- | --- |
| Native Linux | Not measured | Not measured | Not measured | Not recorded / Not recorded / Not recorded |
| Chrome | 6.147 / 8.100 | 0.123 / 0.200 | 0.017 / 0.100 | 120 / 120 / 120 |
| Brave | 6.485 / 8.400 | 0.121 / 0.200 | 0.027 / 0.100 | 120 / 120 / 120 |

### User-reported browser runtime acceptance

The user explicitly confirmed that all manual browser checks passed, including
full close/reopen, import/export and failure paths. This is user-performed
acceptance, separate from the automated/native probe evidence above.

| Readout | Chrome | Brave |
| --- | --- | --- |
| ROM / execution | `sram.gba`, Running; status `Reset sram.gba` | `sram.gba`, Running; status `Loaded sram.gba` |
| Save acknowledgement | Saved revision 19 | Saved revision 13 |
| Instructions | 29477119 | 19232015 |
| GBA cycles | 323911002 | 211330848 |
| Backup | `Some(Sram)` / `Identified(Sram)` / override `None` | `Some(Sram)` / `Identified(Sram)` / override `None` |
| PCM rate / produced | 32768 Hz / 632638 | 32768 Hz / 412755 |
| Staging drops / empty FIFO | 0 / 0 | 0 / 0 |
| Host audio | Off; no audible playback acceptance inferred | Off; no audible playback acceptance inferred |
| Close/reopen | PASS, user confirmed | PASS, user confirmed |
| Import/export | PASS, user confirmed | PASS, user confirmed |
| Failure paths | PASS, user confirmed | PASS, user confirmed |

Save revision numbers are storage revisions, not score values. Exact score,
browser versions, refresh rate, zoom and test duration were not recorded.
The supplied browser evidence does not establish native GUI acceptance.

### Manual commands and acceptance

Native app:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked --release -p gba-app
```

Browser app, opened manually at `http://127.0.0.1:8080` in Chrome and Brave:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

1. Click **Load SRAM score**, tap/release **Z** several times, then **Pause**.
   Count the green cells and wait for **Saved revision** with no pending revision.
2. Fully close/reopen the native app or browser. Click **Load SRAM score** again.
   The same number of cells must return. Use the same browser profile and origin.
3. **Export SRAM**, add another point and save it, then **Import SRAM** using the
   earlier export. Wait for storage acknowledgement, click **Resume**, and check
   that the earlier score returns. Close/reopen and check that score again.
4. Import that export into a different ROM forced to **SRAM**, or import a short
   file. Expect an explicit rejection and unchanged cartridge bytes.
5. While saving, request another ROM. It must wait for the save barrier. Native
   close must also wait; if storage fails, its window must remain open for retry.

Browser failed-write acceptance can be exercised after loading the score by
running this temporary DevTools Console hook. It rejects only read/write
transactions, leaving reads available; it does not alter emulator code or saves.

```javascript
// Retain the platform method so the injected storage failure can be reversed.
const originalSaveTransaction = IDBDatabase.prototype.transaction;
IDBDatabase.prototype.transaction = function (...args) {
  if (args[1] === "readwrite") {
    throw new Error("Manual SRAM acceptance: storage write rejected");
  }
  return originalSaveTransaction.apply(this, args);
};
```

Tap/release **Z**. Expect **Save failed** and a pending revision, with no **Saved**
claim for that revision. **Export SRAM** must still prepare the latest score.
Restore the platform method, click **Retry save storage**, wait for **Saved
revision**, and close/reopen to confirm recovery:

```javascript
// Restore normal IndexedDB transactions before retrying the pending revision.
IDBDatabase.prototype.transaction = originalSaveTransaction;
```

### Plan comparison

- [x] SRAM bytes/protocol and dirty revisions added to cartridge state.
- [x] Native disk and browser IndexedDB routes, ROM identity, initial save loading,
      identity-validated import/export implemented.
- [x] Serialized operations and identity/generation/revision-tagged storage
      acknowledgement implemented; failed writes retain dirty bytes.
- [x] Native process reopen, guest framebuffer and portable image contracts pass.
- [x] Chrome and Brave IndexedDB reopen, download/import and failure paths
      manually accepted by the user.
- [ ] Native GUI/pickers/close/failure status manually accepted; native headless
      process reopen and storage contracts already pass.

Implementation, automated evidence and browser manual acceptance are complete.
The only remaining platform acceptance item is the native GUI/picker/close/failure
check; the supplied confirmation concerns Chrome and Brave. At that closeout,
Slice 14 Flash behavior had not started; its current evidence follows below.

## Slice 14 — Flash64 score recovery (2026-10-01)

Flash64 is selected through the existing centralized detector/typed override.
The cartridge owns 65536 erased (`ff`) bytes, Panasonic identification (`32 1b`),
unlock/command decoding, byte programming, chip erase and 4 KiB sector erase.
Programming clears bits; erase restores them to one. Ordinary save-bus reads
cancel incomplete command sequences while identification reads remain available
until reset/exit. CPU reset retains bytes and pending revisions and clears the
volatile decoder/identification state. Program/erase currently complete
synchronously; physical busy delays and status-bit polling are not modeled.

The pinned guest exposed discarded low address bits on wide CPU stores. Backup
stores now retain those bits so the eight-bit bus selects the correct source
byte; ordinary memory stores retain their alignment behavior. The upstream test
failed case 6 before this fix and passes all eleven cases afterward. One focused
unit regression was added: a read between unlock writes must prevent later
writes from programming a byte. It first failed on the missing Flash bus and
then passed; an uninterrupted sequence programs successfully and survives reset.

Flash snapshots reuse the existing identity, serialized storage, dirty revision,
completion acknowledgement, disk and IndexedDB routes. Initial loads are clean,
imports are dirty, stale acknowledgements cannot clean newer data, and rejected
sizes leave bytes intact. Existing SRAM exports remain byte-for-byte compatible.
Flash exports contain `GBAFLS64`, the 32-byte original-ROM SHA-256 digest and
exactly 65536 backup bytes (65576 total). The core also validates the image against
the currently selected hardware. Shared UI labels now say backup/save.

### Fixture and automated evidence

**Load Flash64 score** loads the original `roms/flash-score.gba` through the normal
app storage route. Its source, frozen identity/bounds, rebuild command and shared
native/WASM contract are in [roms/flash](../roms/flash/README.md). It uses direct
ARM startup without BIOS or test firmware. Tap/release **Z (A)** for a point;
each point fills an eight-by-eight green cell, up to sixteen.

**Load Flash64 test** loads the unchanged upstream diagnostic. Its padded
identification strings lack SDK version digits, so this explicitly configured
fixture uses the existing typed Flash64 override and original test firmware for
its result text. The detector is unchanged and no commercial-game patch is added.
Its manifest requires the exact terminal opcode, zero result, CPSR and complete
success-screen SHA-256; reaching an arbitrary loop is insufficient.

| Evidence | Result |
| --- | --- |
| Score ROM SHA-256 / size | `5474814fb7782e47aab77fc2fb6da5b38a6400843d08e6363e5a9f37548d4e07` / 616 bytes |
| Upstream source revision | `jsmolka/gba-tests` at `a7113b67e63f83a9b321696ddd7042ccfad6c881`; MIT notice retained |
| Upstream ROM SHA-256 / size | `7e2aa32e943aedde88bd750eadcdbf55152d3a1ec61385011b7f15cd85b07c02` / 3708 bytes |
| Upstream terminal/result | PASS: `0x08000ac8`, instruction `eafffffe`, `r12 = 0`, CPSR `600000df` |
| Upstream bounded completion | 20750192 cycles; 1760344 instructions; limits 50000000 cycles / 4000000 instructions |
| Upstream exact success screen | PASS: SHA-256 `59ce42abae9825c2d2579c5cd838e47d88be917e37ea36ff162d46fc5d0991e3` after scanout |
| Score completion | Mailbox `0x03000000 = 0x74`; score at `0x03000002`; three frames, 200000 instructions per frame, at most 842750 cycles |
| Fresh score, A held | Score 1; PC `0x08000148`; 842688 cycles; 78839 instructions; revision 5, dirty |
| Reopened score, A released | Score 1; PC `0x08000150`; 842696 cycles; 78839 instructions; revision 1, clean |
| Restored score, next increment | Score 2; PC `0x08000150`; 842698 cycles; 78841 instructions; revision 4, dirty; erase/reprogram required |
| Exact score output | All 38400 completed pixels verified; backup begins `a5 01` or `a5 02`, remaining 65534 bytes `ff` |
| Portable score-1 image SHA-256 | `0cc173a9820a93c5d32f5881e5cee8b86667f7c6ddd511bc70fce9393de6d5b1` |
| Native full-process reopen | PASS: separate write/read processes using production disk and envelope functions |
| Export/import and failure contract | PASS: disk export/readback, envelope decode, wrong identity rejected, invalid image size rejected, stale revision remains dirty, failed native replacement rejected |
| Production WASM execution | PASS in Node: upstream terminal/result plus original identification, score/recovery, erase/reprogram and image/revision contract; browser storage is not exercised |
| Existing SRAM recovery | PASS: production native write/read probes and existing WASM contract; original export hash unchanged |
| Core/session tests | PASS: 14 core unit, one backup integration and nine session tests (24 total) |
| All existing release fixtures | PASS, including the new pinned Flash64 case |
| Reproducible fixture builds | PASS: existing build-fixtures verifies the pinned binary; `python3 roms/flash/build.py` rebuilds the score with matching hash |
| Clippy / format | PASS: workspace/all targets with warnings denied; `cargo fmt --all -- --check` |
| Native / WASM app builds | PASS: native release build, WASM check and release Trunk build (0.21.14) |
| Native GUI/pickers/close/failure status | PASS: user confirmed all manual acceptance checks complete |
| Chrome / Brave test screen, IndexedDB reopen, export/import and failure UI | PASS: user manually confirmed all checks, including recovery, import/export, diagnostic screen and failure paths |

Retained evidence: [tests](verification/flash-tests.txt),
[release fixtures](verification/flash-fixtures.txt),
[fixture builds](verification/flash-build-fixtures.txt),
[native write](verification/flash-native-write.txt),
[separate-process reopen](verification/flash-native-reopen.txt), and
[production WASM](verification/flash-wasm.txt).
Native probe data is in `/tmp/gba-flash-verified-20261001`; temporary data is not
a permanent evidence store. Live environment: Linux `7.2.7-arch1-1`, rustc/cargo
`1.98.1`. Browser versions, refresh rate and zoom were not recorded for this slice.

### Slice 14 timing fields

All earlier user-supplied millisecond entries are preserved. The following Chrome
and Brave measurements were supplied by the user for `flash-score.gba`, together
with confirmation that all manual acceptance checks passed. Mean values are
recorded as mean, not as p50. Unreported measurements remain unavailable.

| Measurement | Linux app | Chrome | Brave |
| --- | --- | --- | --- |
| Core execution mean / p95 (ms) | Not measured | 7.265 / 9.400 | 6.639 / 9.000 |
| Core execution samples | Not recorded | 120 | 120 |
| Pixel conversion mean / p95 (ms) | Not measured | 0.149 / 0.300 | 0.135 / 0.200 |
| Pixel conversion samples | Not recorded | 120 | 120 |
| Texture submission mean / p95 (ms) | Not measured | 0.022 / 0.100 | 0.017 / 0.100 |
| Texture submission samples | Not recorded | 12 | 120 |
| Core / conversion / submission p50 and p99 (ms) | Not measured | Not measured | Not measured |
| Sustained speed | Not measured | Not measured | Not measured |
| Version / refresh rate / zoom | Not recorded | Not recorded | Not recorded |

The supplied browser runtime snapshots also record:

| Runtime evidence | Chrome | Brave |
| --- | --- | --- |
| Loaded cartridge | flash-score.gba | flash-score.gba |
| Backup selection / detection / override | Some(Flash64) / Identified(Flash64) / None | Some(Flash64) / Identified(Flash64) / None |
| Storage completion | Saved revision 38 | Saved revision 41 |
| Guest state | Running | Running |
| Executed instructions | 26191336 | 20941686 |
| GBA cycles | 287816497 | 230116300 |
| Core PCM rate | 32768 Hz | 32768 Hz |
| Produced PCM samples | 562141 | 449445 |
| Staging drops / empty FIFO | 0 / 0 | 0 / 0 |
| Host audio | Off | Off |

The snapshots show acknowledged save revisions and automatic Flash64 selection.
Manual recovery and failure-path acceptance is based on the user's explicit
confirmation, rather than inferred from these snapshots. CLI completion samples
in retained logs are not sustained GUI benchmarks.

### Manual app commands and acceptance

PASS: the user confirmed all manual checks complete on 2026-10-01. The commands
and procedure below are retained for reproducing acceptance.

Run the browser server from the workspace and open **the same origin** in each
browser for close/reopen checks:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in Chrome and Brave. Use **Auto detect** in the backup
selector. In each browser:

1. Click **Load Flash64 test**; wait for the guest's successful test screen.
2. Click **Load Flash64 score**. Tap/release Z several times and confirm green
   cells increase once per press. Pause and wait for **Saved revision**.
3. Close the browser completely, reopen the same URL, and load the score again.
   Confirm the same cells return; add another point and repeat the save check.
4. Export the save. Change the score, then import that export. Resume after the
   import and confirm the earlier cells return; wait for **Saved revision**,
   close/reopen and confirm the imported score persists.
5. Try an SRAM/different-ROM export or a truncated Flash export. It must report
   an error and retain the score. For the storage failure path, inject an aborted
   write transaction using the DevTools snippet below, then add a point. It must
   remain pending, show failure, and permit Retry/Export. Restore writes with the
   second snippet and confirm Retry reaches **Saved revision**. Record the result.
6. Record the measurements above and browser version/refresh rate/zoom.

DevTools console: abort writes to the emulator's `saves` store before commit:

```javascript
// Preserve the browser method so the fault can be removed without reloading.
globalThis.flashOriginalPut = IDBObjectStore.prototype.put;
IDBObjectStore.prototype.put = function (...args) {
  const request = globalThis.flashOriginalPut.apply(this, args);
  if (this.name === "saves") {
    const transaction = this.transaction;
    queueMicrotask(() => transaction.abort());
  }
  return request;
};
```

Restore the method before clicking **Retry**:

```javascript
IDBObjectStore.prototype.put = globalThis.flashOriginalPut;
delete globalThis.flashOriginalPut;
```

For native visual acceptance, use the same buttons/checks with:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

### Plan comparison

- [x] Flash64 command decoding, identification, erase and program behavior added.
- [x] Flash score uses the completed host save route and the centralized selection path.
- [x] Required pinned `flash64.gba` passes on native and production WASM.
- [x] Flash-backed score survives a separate-process native reopen.
- [x] Ordinary reads cannot accidentally continue a partial command sequence.
- [x] Affected prior tests/fixtures pass; documentation and manual commands updated.
- [x] Native visual/picker/close/failure acceptance confirmed manually by the user.
- [x] Chrome and Brave score reopen, import/export, diagnostic screen and failure UI confirmed manually by the user.

Implementation, automated checks and user-confirmed manual platform acceptance
are complete for Slice 14. Unreported timing/environment fields remain marked
unavailable. Slice 15 banked Flash128 evidence is recorded below.


## Slice 15 — Banked Flash128 score recovery (2026-10-01)

The shared Flash decoder now selects a 64 or 128 KiB physical image from the
centralized cartridge configuration. Flash128 exposes Sanyo identification
`62 13` and the unlocked `B0` bank command with valid selectors zero/one.
Reads and programs address the selected bank; sector erase changes only that
bank, while chip erase covers both. Reset/load returns volatile selection to
bank zero without discarding backup bytes or pending revisions.

Both banks persist through the completed serialized host route. The portable
`GBAFL128` envelope contains the original-ROM SHA-256 and exactly 131,072 backup
bytes (131,112 total bytes). SRAM and Flash64 envelopes remain compatible.
The core rejects a capacity mismatch before mutation; stale completion cannot
acknowledge a newer imported image.

The original [banked score cartridge](../roms/flash-banked/README.md) stores
`a5 score` in bank zero and `5a (score + 16)` in bank one at identical offsets.
Its green and blue bands separately render the two restored records. The
contract verifies all 38,400 completed pixels and all 131,072 backup bytes.
One focused regression was added: programming/sector erase in bank one must
preserve bank zero, and a complete loaded image must restore both. RED failed
with `UnmappedAddress` at the first Flash128 command; GREEN passes with banked
hardware implemented. Existing Flash64 read-interruption coverage is retained.

### Fixture and automated evidence

| Field | Verified result |
|---|---|
| Score ROM SHA-256 / size | `19a9657c138ed726e7ba055f1d73c27aeee432054505bbd9a23a4b210c3e602b` / 868 bytes |
| Startup / selection | Direct ARM, no BIOS; automatic Flash128 via `FLASH1M_V103` |
| Mailboxes | `0x03000000 = 0x75`; bank-zero score at `0x03000002`; bank-one encoded score at `0x03000004` |
| Bounds | Three frames; 200,000 instructions/frame; 843,000 cycles |
| Boundary state | Source-defined polling `0x08000164..0x08000184` or drawing `0x080001c8..0x0800025c`; exact mailboxes, pixels and backup image required |
| Fresh score | Score 1; PC `0x0800023c`; 842,691 cycles; 86,089 instructions; revision 10, dirty |
| Reopened score | Score 1; PC `0x08000240`; 842,693 cycles; 86,091 instructions; revision 1, clean |
| Next update | Score 2; PC `0x08000238`; 842,691 cycles; 86,091 instructions; revision 7, dirty; both bank sectors erased/reprogrammed |
| Upstream identity | Unchanged `jsmolka/gba-tests` revision `a7113b67e63f83a9b321696ddd7042ccfad6c881`; MIT notice retained |
| Upstream ROM SHA-256 / size | `9ac50e51d3ce4209dbdf85e472e70c067d5827e9af1bb3e707f6bd9059d5f0c6` / 4,096 bytes |
| Upstream completion | PC `0x08000c4c`, instruction `0xeafffffe`, r12 = 0, CPSR `0x600000df`; 20,750,196 cycles / 1,760,348 instructions |
| Upstream success framebuffer | `59ce42abae9825c2d2579c5cd838e47d88be917e37ea36ff162d46fc5d0991e3` |
| Core/session tests | PASS: 15 core unit, one backup integration, nine session tests (25 total) |
| Workspace Clippy | PASS, all targets with warnings denied |
| Release fixtures / reproducible builds | PASS, including unchanged upstream Flash64/Flash128 and rebuilt original score |
| Production WASM guest | PASS: upstream Flash128 plus banked score, restore/update and import validation |
| Native persistence | PASS: production disk write and separate-process reopen; full image and identity-bearing export/import checked; failed replacement rejected |
| App build checks | PASS: native release, WASM check, Trunk 0.21.14 release build |
| Live Linux / Chrome / Brave acceptance | PASS: user confirmed all manual checks complete on 2026-10-01 |

Evidence: [tests](verification/flash-banked-tests.txt),
[Clippy](verification/flash-banked-clippy.txt),
[fixture builds](verification/flash-banked-build-fixtures.txt),
[score rebuild](verification/flash-banked-build.txt),
[release fixtures](verification/flash-banked-fixtures.txt),
[WASM](verification/flash-banked-wasm.txt),
[native write](verification/flash-banked-native-write.txt), and
[separate-process reopen](verification/flash-banked-native-reopen.txt).
Native probe data: `/tmp/gba-flash-banked-20261001` (temporary, not durable evidence).

### Timing and environment fields

Chrome and Brave timings below were supplied by the user for
`flash-banked-score.gba`, with 120 samples per metric. Existing timing entries
for earlier slices are preserved. CLI completion samples in logs are bounded
diagnostic runs; they are not sustained application timings.

| Field | Linux | Chrome | Brave |
|---|---|---|---|
| Core frame mean / p95 (ms) | Not measured | 7.253 / 9.500 | 6.153 / 8.700 |
| Pixel conversion mean / p95 (ms) | Not measured | 0.144 / 0.200 | 0.132 / 0.200 |
| Texture upload mean / p95 (ms) | Not measured | 0.023 / 0.100 | 0.011 / 0.100 |
| Samples core / conversion / upload | Not recorded | 120 / 120 / 120 | 120 / 120 / 120 |
| Sustained unthrottled speed | Not measured | Not measured | Not measured |
| Save/reopen and both colored bands | PASS, user confirmed | PASS, user confirmed | PASS, user confirmed |
| Browser version / environment | Not applicable | Not recorded for this slice | Not recorded for this slice |

### User-supplied runtime snapshots

| Field | Chrome | Brave |
|---|---|---|
| Loaded cartridge | flash-banked-score.gba | flash-banked-score.gba |
| Backup selection / detection / override | Some(Flash128) / Identified(Flash128) / None | Some(Flash128) / Identified(Flash128) / None |
| Storage acknowledgment | Saved revision 148 | Saved revision 70 |
| Instructions | 32,185,605 | 33,147,311 |
| GBA cycles | 326,219,301 | 335,963,753 |
| Execution state | Running | Running |
| Core PCM rate | 32,768 Hz | 32,768 Hz |
| Produced PCM samples | 637,147 | 656,179 |
| Staging drops / empty FIFO | 0 / 0 | 0 / 0 |
| Host audio | Off; Enable audio available | Off; Enable audio available |

These snapshots record acknowledged storage revisions and automatic Flash128
selection. Manual diagnostic, both-bank recovery, export/import, rejected-image,
and failed-write/retry acceptance is based on the user's explicit confirmation
that all manual checks were completed, rather than inferred from the counters.
Browser versions, native timings and sustained unthrottled speed were not supplied.

### Manual commands and acceptance

PASS: the user confirmed all manual checks complete on 2026-10-01. The following
commands and procedure are retained for reproducing acceptance.

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

Browser server (use the same origin for save and reopen):

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in Chrome and Brave. For each target:

1. Choose **Auto detect**, click **Load Flash128 test**, and confirm the successful
   guest test screen. The pinned diagnostic uses the validated Flash128 override.
2. Click **Load banked Flash128 score**. Confirm automatic Flash128 selection.
   Tap/release Z several times: green and blue bands must have the same cell count.
3. Wait for acknowledged storage success, close the entire app/tab, reopen on
   the same origin and load the same score ROM. Both bands must retain their count.
4. Export the backup, increment again and wait for success, then import the old
   export. Both bands must return to the exported count and survive another reopen.
5. Reject truncated, Flash64 or different-ROM exports without changing either bank.
   Reuse the Slice 14 transaction-abort procedure above: a failed write must remain
   pending, and retry must acknowledge only a successful storage completion.
6. Record the millisecond fields above and any live acceptance/environment evidence.

### Plan acceptance checklist

- [x] Flash128 bank selection and physical capacity implemented.
- [x] Programming/sector erase in one bank preserves the other; full-image restore verified.
- [x] Required unchanged upstream `flash128.gba` passes on native and production WASM.
- [x] Distinct records in both banks survive separate-process native close/reopen.
- [x] Existing save route reused; exact-capacity validation and revision behavior verified.
- [x] Live Linux visual acceptance (user confirmed).
- [x] Chrome and Brave close/reopen, export/import and failed-write acceptance (user confirmed).

Slice 15 is complete against the plan: automated checks pass and the user
confirmed all manual target checks. Unreported measurements/environment fields
remain explicitly unavailable. Slice 16 EEPROM evidence is recorded below.

## Slice 16 — EEPROM score recovery (2026-10-01)

The cartridge now implements the EEPROM serial bus for 512-byte and 8-KiB
hardware. Halfword accesses clock bit zero, with MSB-first command, address and
data fields. Read responses contain four dummy clocks followed by 64 data
clocks; writes replace one eight-byte block and return ready. Programming is
synchronous, as in the current Flash abstraction; physical write-busy duration
is not modeled. EEPROM is mapped in region `0D`; ROMs larger than 16 MiB retain
ordinary ROM reads except in the final 256-byte EEPROM window.

`EEPROM_V` retains family-identification evidence. Valid DMA3 command lengths
resolve six versus fourteen address clocks (9/73 versus 17/81 halfwords).
Only a complete command with a valid stop bit resolves capacity or changes
bytes. Large EEPROM uses the low ten address bits for 1024 physical blocks.
`Eeprom512` and `Eeprom8k` are explicit typed overrides, also parsed as
`eeprom512` and `eeprom8k`. Already resolved capacity cannot silently change.
A validated 512-byte or 8192-byte initial save can resolve an unknown capacity
before guest execution. Until resolution, the clean, empty snapshot exposes
initial-load/import storage but portable export remains disabled.

The existing serialized disk/IndexedDB route is reused. Portable `GBAEEPR1`
exports contain the original ROM SHA-256 and exactly 512 or 8192 backup bytes
(552 or 8232 total). Core imports reject truncation or a mismatch with the
resolved/overridden capacity before mutation. Imported bytes remain dirty;
stale acknowledgments cannot clean newer revisions. Earlier save envelopes
remain compatible.

The two [original score cartridges](../roms/eeprom/README.md) use one assembly
source with six/fourteen address clocks. They store distinct records in the
first and last physical blocks. Green cells show the last-block score, blue
cells show the independently restored first-block score. The contract checks
all 38,400 pixels and every save byte, including untouched erased blocks.

One focused core regression was added. It catches EEPROM DMA stores being
unmapped, capacity/address aliasing, broken serial readback and lost restored
blocks. RED failed at the first serial write with `UnmappedAddress` at
`0x0d000000`, width 2. GREEN passes for both address widths. Original guest
contracts supply the slice acceptance checks rather than additional duplicate
unit tests. The first host probe also rejected the EEPROM-sized export through
the prior decoder; the new envelope and production probes pass.

### Fixture and automated evidence

| Field | 512-byte cartridge | 8-KiB cartridge |
|---|---|---|
| ROM | `eeprom512-score.gba` | `eeprom8k-score.gba` |
| ROM SHA-256 | `0d755a8bc3fd69600304d5115a234c6356c51d1b36faecf68e9f4bf95cd65c60` | `28629d0241c6861acdb38227f47a7af5926bc1ae12a64880c01a4c08263bce58` |
| ROM bytes / save bytes | 1044 / 512 | 1048 / 8192 |
| Startup / identification | Direct ARM, no BIOS; `EEPROM_V124`, automatic six-bit detection | Direct ARM, no BIOS; `EEPROM_V124`, automatic fourteen-bit detection |
| Fresh score 1: PC / cycles / instructions | `0x0800020c` / 842,693 / 86,168 | `0x08000218` / 842,689 / 86,185 |
| Fresh revision / dirty | 4 / true | 4 / true |
| Reopened score 1: PC / cycles / instructions | `0x08000218` / 842,701 / 85,974 | `0x08000208` / 842,693 / 85,973 |
| Reopened revision / dirty | 1 / false | 1 / false |
| Updated score 2: PC / cycles / instructions | `0x0800020c` / 842,689 / 86,072 | `0x08000218` / 842,691 / 86,080 |
| Updated revision / dirty | 3 / true | 3 / true |
| Exact restored records | First `5a 11 00 00 00 00 00 00`; last `a5 01 00 00 00 00 00 00` | Same records at physical offsets 0 and 8184 |
| Bounds | Three frames; 200,000 instructions/frame; 843,000 cycles | Same |
| Completion evidence | `0x03000000 = 0x76`; score at `0x03000002`; encoded first-block score at `0x03000004`; exact pixels/bytes | Same |
| Boundary PC | Source-defined polling `0x08000134..0x0800018c` or drawing `0x0800019c..0x0800022c` | Same |
| Native persistence | PASS: production write and separate-process reopen; portable identity round-trip and failed replacement rejection | PASS: same |
| Production WASM guest | PASS: serial read/write, restore/update, distinct blocks and import/override validation | PASS: same |
| Live Linux visual / Chrome / Brave | PASS: user confirmed all manual checks | PASS: user confirmed all manual checks |

Shared validation: PASS, 16 core unit tests, one backup integration test and
nine session tests (26 total); workspace Clippy with all targets and warnings
denied; all prior release fixtures; reproducible prior fixture build and both
original EEPROM rebuilds; native release app; WASM app check; Trunk release
bundle. The WASM contract executes production core/session code in Node,
without launching a browser or proving IndexedDB behavior.

Evidence: [core/session tests](verification/eeprom-tests.txt),
[Clippy](verification/eeprom-clippy.txt),
[prior builds](verification/eeprom-build-fixtures.txt),
[EEPROM rebuilds](verification/eeprom-build.txt),
[prior fixtures](verification/eeprom-fixtures.txt),
[WASM contract](verification/eeprom-wasm.txt),
[512-byte native write](verification/eeprom-native-write.txt),
[512-byte reopen](verification/eeprom-native-reopen.txt),
[8-KiB native write](verification/eeprom-native-large-write.txt),
[8-KiB reopen](verification/eeprom-native-large-reopen.txt),
[WASM app check](verification/eeprom-app-wasm.txt), and
[Trunk release](verification/eeprom-web-build.txt).
Native probe data is in `/tmp/gba-eeprom-20261001` (temporary).

### Timing and environment fields

The user supplied the following Chrome and Brave timings for
`eeprom512-score.gba`, with 120 samples per metric. Earlier timing entries are
preserved. Native and 8-KiB cartridge timings were not supplied; CLI diagnostic
completion durations are not sustained app measurements.

| Field | Linux | Chrome | Brave |
|---|---|---|---|
| 512-byte ROM core frame mean / p95 (ms) | Not measured | 6.703 / 10.000 | 7.257 / 9.600 |
| 512-byte ROM pixel conversion mean / p95 (ms) | Not measured | 0.142 / 0.300 | 0.150 / 0.300 |
| 512-byte ROM texture submission mean / p95 (ms) | Not measured | 0.030 / 0.100 | 0.017 / 0.100 |
| 512-byte ROM samples core / conversion / submission | Not recorded | 120 / 120 / 120 | 120 / 120 / 120 |
| 8-KiB ROM core / conversion / submission timings | Not measured | Not measured | Not measured |
| 8-KiB ROM sample counts | Not recorded | Not recorded | Not recorded |
| Sustained unthrottled speed | Not measured | Not measured | Not measured |
| Both sizes: save/reopen, both bands, import/export, failure/retry | PASS, user confirmed | PASS, user confirmed | PASS, user confirmed |
| Environment | Linux 7.2.7-arch1-1; Rust/Cargo 1.98.1; ARM assembler 2.47.20260726; Node v26.10.0 | Version/environment not recorded | Version/environment not recorded |

### User-supplied runtime snapshots

| Field | Chrome | Brave |
|---|---|---|
| Loaded cartridge | eeprom512-score.gba | eeprom512-score.gba |
| Backup selection / detection / override | Some(Eeprom) / Identified(Eeprom) / None | Some(Eeprom) / Identified(Eeprom) / None |
| EEPROM capacity | 512 bytes | 512 bytes |
| Storage status | Backup restored | Saved revision 28 |
| Instructions | 13,831,834 | 36,753,045 |
| GBA cycles | 140,135,053 | 372,391,632 |
| Execution state | Running | Running |
| Core PCM rate | 32,768 Hz | 32,768 Hz |
| Produced PCM samples | 273,701 | 727,327 |
| Staging drops / empty FIFO | 0 / 0 | 0 / 0 |
| Host audio | Off; Enable audio available | Off; Enable audio available |

These snapshots identify the 512-byte cartridge and its measured timings.
Acceptance for both EEPROM sizes, including native visual checks, browser
reopening, import/export, overrides and failure/retry, is based on the user's
explicit confirmation that everything was checked manually. No timing from
the 512-byte cartridge is attributed to the 8-KiB cartridge.

### Manual commands and acceptance

PASS: the user confirmed all manual checks complete for both EEPROM sizes.
Commands and checks below are retained for reproducing acceptance.

Native app:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

Browser server (keep the same origin across save/reopen):

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in Chrome and Brave. Repeat for **Load EEPROM
512-byte score** and **Load EEPROM 8-KiB score**:

1. Choose **Auto detect**, load the score, and confirm EEPROM capacity is 512
   or 8192 bytes. Tap/release Z several times; both bands must have equal counts.
2. Wait for the saved revision acknowledgment. Close the entire app/tab, reopen
   on the same origin and load the same ROM. Both bands must retain their counts.
3. Export, increment again and wait for saved acknowledgment. Import the older
   export, resume, and confirm both bands return to the exported count. Reopen
   again to check the imported state was stored.
4. Reject a truncated export and an export from the other EEPROM ROM without
   changing the score. Select the matching `Eeprom512`/`Eeprom8k` override,
   reload, and confirm correct restore. Return to **Auto detect** afterward.
5. Use the Slice 14 IndexedDB transaction-abort procedure to fail a save. It
   must remain pending; restore the method and click **Retry**, then reopen to
   confirm only the successful write was acknowledged.
6. Enter separate millisecond measurements for each ROM, and report browser
   versions when available. Manual acceptance is complete; unsupplied
   measurements and environment details remain explicitly unavailable.

### Plan acceptance checklist

- [x] EEPROM command/address/data behavior for both supported sizes.
- [x] Tested automatic size detection, signature evidence and validated explicit overrides.
- [x] Original fixtures with frozen identities and bounded exact-output contracts.
- [x] Both capacities pass serial read/write, restore/update and separate-process native reopening.
- [x] Existing persistence route reused; imports validate capacity before mutation.
- [x] Previous regression gates pass; performance evidence and manual commands updated.
- [x] User-performed native visual acceptance (user confirmed).
- [x] Chrome and Brave persistence, visual, import/export and failed-save/retry acceptance for both sizes (user confirmed).

Slice 16 is complete against the plan: automated checks pass and the user
confirmed all manual platform acceptance checks for both EEPROM sizes.
Unsupplied measurements/environment fields remain explicitly unavailable.
No Slice 17 implementation was included.

## Slice 17 — Pulse melody (2026-10-01)

Implementation, automated plan checks and user-performed native/Chrome/Brave
acceptance pass. The user explicitly confirmed all manual checks complete and
supplied the runtime snapshots and timings below. Earlier slice records are preserved.

### Implementation and acceptance evidence

Both pulse voices run on the master clock, with eight-step duty oscillators,
length counters and 64 Hz envelopes. Channel 1 adds 128 Hz sweep scheduling,
frequency-overflow shutdown and trigger state. The 512 Hz sequencer and duty
edges participate in the existing hardware event scheduler. PSG routing and
volume are mixed with Direct Sound before DAC saturation/PWM sampling. PCM
output drains preserve oscillator, envelope, sweep, PWM and resampler history.
Master disable clears PSG state while preserving the Direct Sound devices.
Byte writes merge against writable pulse latches, retaining write-only frequency
bits when the high trigger byte is written.

The original `roms/pulse.gba` scene changes notes every 32 VBlanks and shows a
red/green/blue/white square. The lead plays on the right and the accompaniment
on the left. Z adds the existing FIFO/DMA PCM tone to both sides. The app exposes
**Load pulse melody**, source note values and the control legend.

| Evidence | Result |
|---|---|
| ROM identity / size | `be3429a0ec75d688765fce04cae070df3369367fb2f8f685815dcd5d1b4f1eed` / 756 bytes |
| Startup | Skip BIOS; explicit existing test firmware enabled in app and contract |
| Guest completion | Mailbox `0x03000000 = 0x77`; 184 VBlanks; HALT in declared `0x080000fc..=0x0800010c` range |
| Bounded run | 51,684,864 cycles; 7,810 instructions (budget below 500,000) |
| Recorded signal | 100,947 stereo frames at 32,768 Hz; zero staging drops and FIFO underruns |
| PCM16 WAV SHA-256 | `7edf6d0743f84a87166ac13322b2337bde407009965e44dd5829579db3fa641e` |
| Lead source frequencies | 1536 / 1642 / 1707 / 1792: 256 / approximately 322.8 / 384.4 / 512 Hz |
| Second voice source frequencies | 1024 / 1236 / 1366 / 1536: 128 / approximately 161.4 / 192.2 / 256 Hz |
| Note timing / scanout | Exact mailbox/VBlank counts, note latches, both measured signal periods and all 38,400 pixels checked at frames 4/36/68/100 |
| Scripted controls | Recorded decreasing sweep, 25% duty and fading envelope; master silence and release/retrigger status checked |
| Chunk invariance | Exact sample equality for 280,896-cycle and 997-cycle execution/drain chunks |
| Resampler history | Exact whole/137-sample chunk equality for both voices at 44,100 / 48,000 / 96,000 Hz |
| Native / WASM signal contracts | PASS, same production core and guest contract |
| Focused RED → GREEN | One new test failed because pulse voices were absent; passes with independent routing, FIFO mixing, two sweep/envelope steps and identical drain schedules |
| Regression gates | Core/session tests, complete existing fixture suite, workspace Clippy with warnings denied, native/WASM app checks, release Trunk build: PASS |
| Environment | Linux 7.2.7-arch1-1; Rust/Cargo 1.98.1; browser versions not recorded |

Evidence: [ROM rebuild](verification/pulse-build.txt),
[native/WASM guest and signal contract](verification/pulse-contract.txt),
[core/session tests](verification/pulse-tests.txt),
[Clippy](verification/pulse-clippy.txt),
[native app check](verification/pulse-app-native.txt),
[WASM app check](verification/pulse-app-wasm.txt),
[existing fixtures](verification/pulse-fixtures.txt), and
[release web build](verification/pulse-web-build.txt).
The retained [fixture manifest](../roms/pulse/manifest.toml) freezes identity,
startup, completion bounds, frequencies and recording identity. Recreate a WAV
with the command below; `/tmp/pulse.wav` is temporary, not retained evidence.

### Timing and manual acceptance fields

| Field | Linux | Chrome | Brave |
|---|---|---|---|
| Core frame mean / p95 (ms) | 0.852 / 1.096 | 1.375 / 1.900 | 1.326 / 1.800 |
| Pixel conversion mean / p95 (ms) | 0.118 / 0.149 | 0.271 / 0.400 | 0.261 / 0.400 |
| Texture submission mean / p95 (ms) | 0.024 / 0.030 | 0.046 / 0.100 | 0.040 / 0.100 |
| Samples core / conversion / submission | 120 / 120 / 120 | 120 / 120 / 120 | 120 / 120 / 120 |
| Sustained unthrottled speed | Not measured | Not measured | Not measured |
| Audible two-voice tune alongside PCM | PASS, user confirmed | PASS, user confirmed | PASS, user confirmed |
| Duty, envelope, sweep, disable/reset controls | PASS, user confirmed | PASS, user confirmed | PASS, user confirmed |
| Audible chunk continuity / pause-resume and focus loss/return | PASS, user confirmed | PASS, user confirmed | PASS, user confirmed |
| Browser version / output device / sample rate | Not applicable / not recorded / 48,000 Hz | Not recorded / not recorded / 48,000 Hz | Not recorded / not recorded / 48,000 Hz |

### User-supplied runtime snapshots

| Field | Linux native | Chrome | Brave |
|---|---|---|---|
| Scene | Pulse melody | Pulse melody | Pulse melody |
| Host output rate / state | 48,000 Hz; output active | 48,000 Hz; running | 48,000 Hz; running |
| Audio queue current / maximum / cap (ms) | 74.1 / 78.5 / 80 | 68.2 / 80.0 / 80 | 69.1 / 80.0 / 80 |
| Host underrun frames | 0 | 25 | 51 |
| Host overflow frames | 0 | 2,894 | 1,961 |
| Host output frames | 1,192,448 | 7,094,503 | 1,457,357 |
| Device errors | 0 | Not recorded | Not recorded |
| Core PCM rate | 32,768 Hz | 32,768 Hz | 32,768 Hz |
| Produced core PCM samples | 758,101 | 4,915,508 | 389,218 |
| Core staging drops / empty FIFO | 0 / 0 | 0 / 0 | 0 / 0 |
| Pulse source notes: lead / second | 1792 / 1536 | 1792 / 1536 | 1707 / 1366 |
| Instructions | 55,701 | Not recorded | Not recorded |
| GBA cycles | 388,147,895 | Not recorded | Not recorded |
| Execution state | Running | Not recorded | Not recorded |
| Backup selection / detection / override | None / Unknown / None | None / Unknown / None | None / Unknown / None |

Manual acceptance is based on the user's explicit confirmation that every check
was performed on all three platforms. The snapshots record nonzero browser host
underrun/overflow counters; those values are preserved and are not described as
zero-loss playback. Core staging drops and empty FIFO counts are zero on all
three platforms. The snapshots have different run durations and lifecycle
histories, so cumulative counts are not treated as directly comparable rates.
Browser versions, output device names, elapsed capture durations and sustained
unthrottled speed were not supplied and remain unavailable.

### Manual commands

PASS: all manual checks are user confirmed. Commands and checks below are
retained for reproducing acceptance.

Native app:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

Chrome/Brave server:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in each browser. Click **Load pulse melody**, then
**Enable audio**. Use headphones to distinguish lead right and second voice
left. Confirm repeating red/green/blue/white note changes and a steady tune.
Hold Z to add PCM without disrupting the melody. While holding Z, exercise X
(sweep), Backspace (25% duty), Up (fade) and Down (mute), one at a time. Release
each control to restart the voices. Confirm no unexpected clicks at ordinary
output chunks, and stable behavior through pause/resume and focus loss/return.
Record millisecond timings, browser version, output device and host sample rate
when available; report manual acceptance separately from those measurements.

Optional recorded-signal reproduction:

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 roms/pulse/build.py
python3 roms/pulse/verify.py
cargo run --locked -p gba-tools --release -- verify-pulse --capture /tmp/pulse.wav
```

### Plan acceptance checklist

- [x] Both pulse channels play a melody and second voice through an original guest scene.
- [x] Duty/envelope, enable/retrigger and channel-1 sweep have visible app controls and scripted signal checks.
- [x] Note timing and recorded frequency changes checked independently from synthesis.
- [x] Pulse synthesis stays correct alongside the existing FIFO PCM signal.
- [x] Synthesis and production resampler history preserved across output chunks.
- [x] Existing regression/build gates pass; performance evidence updated with user-supplied timings.
- [x] User-performed audible native acceptance (user confirmed).
- [x] User-performed Chrome and Brave audible/control acceptance (user confirmed).

Slice 17 is complete against the plan: automated checks pass and the user
confirmed all manual native, Chrome and Brave checks. Unreported environment
and sustained-speed fields remain explicitly unavailable. No wave-channel or
noise-channel implementation is included.


## Slice 18 — Wave-channel effect (2026-10-01)

### Scope and behavior

Channel 3 now shares the existing master-clock event scheduler, 512 Hz PSG
sequencer and stereo PWM mixer with both pulse channels and Direct Sound.
It plays high-nibble-first wave RAM, supports 32/64-digit playback and live
opposite-bank CPU access, frequency/restart, length, DAC disable, normal
volume codes and forced 75% volume. Master disable clears PSG controls while
preserving wave RAM. Byte writes merge against writable frequency/length
latches. Oscillator, PWM and resampler history survives output drains.
Bank/access and register masks follow the channel-3 section of
[GBATEK](https://mgba-emu.github.io/gbatek/).

The original `roms/wave.gba` guest fills both wave banks and selects effects
through scripted buttons. Pulse accompaniment plays left, wave plays right,
and Z adds the existing DMA/FIFO PCM to both sides. The app offers
**Load wave effect**, enables the fixture's explicit test firmware, and shows
the control legend. The visible square follows accompaniment notes.
No noise channel or Slice 19 work is included.

### Automated acceptance evidence

| Check | Result |
| --- | --- |
| Focused RED → GREEN | One new regression first failed with `bank zero must play` because wave synthesis was absent; GREEN covers bank isolation/alternation, drain continuity, gain, length, DAC/master disable and RAM retention |
| Native and production WASM guest contract | PASS, same bounded 48-frame script |
| Execution partitions | Frame-sized versus 997-cycle chunks: exactly identical stereo PCM |
| Recorded frequencies | 8192 Hz first effect, 4096 Hz second effect/bank; exact rising-edge spacing |
| Registers and controls | PASS: bank readback, 64-digit mode, volume/status/masks, nonzero low-frequency byte retained by high-byte trigger, mute/restart |
| Mixed channels | PASS: pulse accompaniment and wave, plus existing timer/DMA FIFO PCM |
| Full scanout and completion | PASS: expected full-frame square/background, mailbox 0x77, VBlank count 48, HALT PC 0x08000100–0x08000110 |
| Execution | 13,483,008 cycles; 3,629 instructions (limit 200,000) |
| PCM | 26,334 stereo frames at 32,768 Hz; zero staging drops and zero empty FIFO pops |
| Production resampling | Exact whole/137-sample-chunk equality at 44.1, 48 and 96 kHz on both sides |
| Core/session tests | PASS: 28 tests; one new focused test |
| Existing pulse contract | PASS: both 184-frame execution partitions, zero staging drops/empty FIFO |
| Existing fixture suite | PASS; mixer fixture retains its two deliberately induced empty FIFO pops |
| Workspace Clippy | PASS with warnings denied |
| Native/WASM app checks | PASS |
| Release web build | PASS |

Frozen ROM SHA-256: `e3d7e45940c70aef8897984d9690a9ff90dc5d58da00f271f201b073c05020e4`.
Frozen accepted PCM16 WAV SHA-256: `9add02cf4f7939f791550be69656a345a4a01368872dbf1a4225f03711c96b55`.
The [manifest](../roms/wave/manifest.toml) records startup, mailbox, completion,
bounds and scripted controls. WAV capture is reproducible with the command
below; `/tmp/wave.wav` is temporary.

Retained evidence: [ROM rebuild](verification/wave-build.txt),
[native/WASM guest and signal contract](verification/wave-contract.txt),
[core/session tests](verification/wave-tests.txt),
[pulse regression](verification/wave-pulse-regression.txt),
[existing fixtures](verification/wave-fixtures.txt),
[Clippy](verification/wave-clippy.txt),
[native check](verification/wave-app-native.txt),
[WASM check](verification/wave-app-wasm.txt),
and [release web build](verification/wave-web-build.txt).

### Manual acceptance and performance

PASS: the user explicitly confirmed all manual Slice 18 checks on native Linux,
Chrome and Brave. This includes audible effect selection, bank/64-digit/volume
controls, pulse/PCM mixing, mute/restart, ordinary playback continuity, the
visible scene, pause/resume and focus loss/return.

The following measurements are transcribed from the user's platform snapshots.
Each timing series contains 120 samples; mean and p95 are reported in milliseconds.

| Field | Native Linux | Chrome | Brave |
| --- | --- | --- | --- |
| Audible wave controls/mixing and ordinary chunk continuity | PASS — user confirmed | PASS — user confirmed | PASS — user confirmed |
| Pause/resume and focus loss/return | PASS — user confirmed | PASS — user confirmed | PASS — user confirmed |
| Core execution mean / p95 (ms) | 1.042 / 1.262 | 1.241 / 1.500 | 1.237 / 1.500 |
| Pixel conversion mean / p95 (ms) | 0.133 / 0.159 | 0.231 / 0.300 | 0.229 / 0.300 |
| Texture submission mean / p95 (ms) | 0.027 / 0.043 | 0.031 / 0.100 | 0.035 / 0.100 |
| Samples per timing series | 120 | 120 | 120 |
| Host sample rate (Hz) | 48000 | 48000 (running) | 48000 (running) |
| Current audio queue (ms) | 74.0 | 55.0 | 64.4 |
| Maximum audio queue (ms) | 80.0 | 75.4 | 80.0 |
| Audio queue capacity (ms) | 80 | 80 | 80 |
| Host underrun frames | 0 | 0 | 118 |
| Host overflow frames | 236 | 0 | 15125 |
| Host output frames | 834560 | 857600 | 2523786 |
| Native device errors | 0 | Not applicable | Not applicable |
| Core PCM rate (Hz) | 32768 | 32768 | 32768 |
| Produced core PCM frames | 390749 | 674568 | 1800086 |
| Core staging drops / empty FIFO | 0 / 0 | 0 / 0 | 0 / 0 |
| Executed instructions | 34542 | Not recorded | 177761 |
| GBA cycles | 200063749 | Not recorded | 921644125 |
| Execution state | Running | Not recorded | Running |
| Backup selection / detection / override | None / Unknown / None | None / Unknown / None | None / Unknown / None |
| Output device name | Not recorded | Not recorded | Not recorded |
| Browser version | Not applicable | Not recorded | Not recorded |
| Sustained unthrottled speed / 30-minute measurement | Not measured | Not measured | Not measured |

All three reported core p95 values are below the plan's 12 ms target. Current
queue depths are within the planned 40–80 ms range. These snapshots do not
establish sustained unthrottled speed or a measured 30-minute latency result.

Manual acceptance is attributed to the user's explicit confirmation. Native
and Brave have nonzero host overflow counters, and Brave also has nonzero host
underruns; these are preserved and are not described as zero-loss playback.
Core staging drops and empty FIFO counts are zero on all three platforms.
Capture durations and lifecycle histories were not supplied, so cumulative
host counters are not compared as rates. Browser versions and device names
remain unavailable. All earlier slice timing entries are preserved.

### Manual commands

All manual checks are user confirmed; commands below are retained for reproduction.

Native:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

Chrome/Brave server:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in each browser. Click **Load wave effect**, then
**Enable audio**. With headphones, confirm pulse accompaniment left and wave
right. Hold Z for the second waveform plus PCM on both sides; while held,
exercise X (bank 1), Backspace (64 digits), Up (75% wave volume) and Down
(master mute). Release each control to restart. Check the changing square,
continuous ordinary playback, pause/resume and focus loss/return. Record app
millisecond values and report acceptance separately; provide browser version,
audio device/rate and host queue counters if available.

Recorded-signal reproduction:

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 roms/wave/build.py
python3 roms/wave/verify.py
cargo run --locked -p gba-tools --release -- verify-wave --capture /tmp/wave.wav
```

### Plan acceptance checklist

- [x] A button selects an effect whose guest writes wave RAM.
- [x] Wave playback and applicable bank/access rules are implemented and checked.
- [x] Wave output mixes with both existing pulse voices and PCM.
- [x] Scripted register changes and recorded signal are checked natively and in WASM.
- [x] Core output and supported-rate resampling preserve history across chunks.
- [x] Native/Chrome/Brave listening and lifecycle acceptance — user confirmed.

Slice 18 is complete against the plan: implementation and automated checks pass,
and all manual native/Chrome/Brave acceptance checks are user confirmed. The
reported timing and audio snapshots are recorded above; unavailable environment
and sustained-speed measurements remain explicitly marked.

## Slice 19: Noise effect and paired timer/FIFO scene events

Implemented and checked against `plan_final.md` section 19 on 2026-10-01.
Automated native/WASM signal checks and the native device probe pass. The user
explicitly confirmed that all manual checks work on native Linux, Chrome and
Brave. Slice 19 is accepted with that user-attributed manual evidence.
Earlier user-entered millisecond measurements are preserved.

### Implementation and fixture evidence

Channel four now implements seven-/fifteen-stage polynomial noise, divisor/shift
clocking, trigger, envelope, length, DAC/master disable, readable masks, byte-write
latches and the PSG activity flag. It participates in the existing event-clocked
PWM mixer alongside both pulse voices, wave and both Direct Sound FIFOs. Device
history survives ordinary PCM drains. Noise parameters follow the
[GBATEK channel-four register specification](https://mgba-emu.github.io/gbatek/#gba-sound-channel-4---noise).

DMA2 now uses the same sound-mode descriptor and independent FIFO request path
as DMA1. The original `noise.gba` scene primes both FIFOs, keeps all implemented
voices running, and uses timer 0 at /2048 followed by timer 1 /2, timer 2 /3 and
timer 3 /4 cascades. Paired red/green square events trigger timed noise and swap
FIFO A/B timer selection. Z changes the event immediately; X selects seven-bit
noise; Backspace isolates noise while all device clocks continue.

| Bounded guest evidence | Result |
| --- | --- |
| Startup | Skip BIOS with explicit test firmware; no Slice 20 work |
| ROM bytes / SHA-256 | 772 / `6a66651b77d105b4d1bc646c4dc5bd2df8c881dc8aa51bc3e9a3741a5daa0de0` |
| Mailbox | ID `0x79`; exact VBlank count; latched input/phase and selected FIFO clocks |
| Terminal state | HALT, PC within `0x08000198..=0x080001a8` |
| Frames / cycles / instructions | 1,800 / 505,612,800 / 54,506 |
| Emulated duration | About 30.137 seconds |
| Core rate / produced stereo frames | 32,768 Hz / 987,525 |
| Staging drops / empty FIFO reads | 0 / 0 |
| Timer cascades | All four counters checked as a mixed-radix clock; all timer IF flags latch |
| DMA1/DMA2 | Independent repeat refill; both FIFO destinations checked in core regression |
| Alternate FIFO clocks | Recorded sign-transition spacing swaps 8/16 PCM samples between sides |
| Noise | Envelope decay, one-tick/full-length expiry, both widths, DAC/master disable and masks checked |
| Short-mode recorded period | 127 noise edges × 4,096 cycles = 1,016 PCM samples |
| Scanout | All 38,400 pixels checked at paired-event/control checkpoints |
| Drain schedules | Exact stereo equality at frame-sized and 997-cycle advances |
| Host resampler rates | 44,100 / 48,000 / 96,000 Hz; exact counts and whole/137-sample chunk equality |
| PCM16 stereo WAV SHA-256 | `eae8cd550ab304a0e533d9bde88299a9da717ed71ccee62634b4336dc1712792` |

The WAV is a core signal recording. It does not prove audible host playback or
absence of audible aliasing. Scripted effects use a 4,096 Hz noise edge clock;
manual listening is user-confirmed on all three platforms at the reported
48,000 Hz host rate. Separate live-device runs at 44.1/96 kHz were not recorded.
No chunk discontinuity or audible aliasing problem was reported, so no unrelated
resampler or mixer rewrite was made.

One focused noise regression was added and the existing FIFO regression was
extended to DMA2. RED failed because the noise activity flag was absent and
DMA2 never supplied PCM; GREEN now checks the real signal and device behavior.
No artificial failures or redundant per-function tests were added.

### Verification

- PASS: [core/session tests](verification/noise-tests.txt): 20 core tests including the integration test, 9 session tests.
- PASS: [workspace Clippy, all targets, warnings denied](verification/noise-clippy.txt).
- PASS: [native app check](verification/noise-native-check.txt) and [WASM app check](verification/noise-wasm-check.txt).
- PASS: [native and production WASM guest/signal contract](verification/noise-signal.txt), including frozen ROM/WAV identities and supported-rate streaming.
- PASS: [release web build](verification/noise-web-build.txt).
- PASS: existing `fixtures run`, including CPU, display, timers/PCM, mixer and Flash diagnostics; existing pulse and wave native/WASM contracts retain their frozen WAV identities.
- PASS: final scoped diff and whitespace check; no BIOS/startup implementation or unrelated changes.

### Native device probe and manual measurements

The [headless native probe](verification/noise-native-audio.txt) exercises the
actual session, resampler, CPAL output and sound device for 30 seconds plus two
seconds of resumed playback. It includes pause/resume, reset and focus
suspension. It does not open or verify a native window.

The probe's final/max queue was 67.6/72.6 ms, with 1,428,480 host output frames,
zero underruns/overflows/device errors, and core counters 884,619/0/0
(produced/staging drops/empty FIFO). These automated probe values are retained
separately from the user's application snapshots below.

PASS — user confirmed all manual Slice 19 checks on native Linux, Chrome and
Brave. This includes paired red/green events, fading noise with pulse/wave/PCM
mixing, Z alternate events, X short-mode noise, Backspace isolation, ordinary
playback continuity, mute, pause/resume, focus loss/return and reset. The user
reported that everything works; no audible aliasing, clicks or growing delay
were reported. Session durations and a measured long-session trend were not
supplied.

All values below are transcribed from the user's platform snapshots. Each of
the three timing series contains 120 samples; mean/p95 values are milliseconds.

| Field | Native Linux | Chrome | Brave |
| --- | --- | --- | --- |
| Manual scene/listening/lifecycle acceptance | PASS — user confirmed | PASS — user confirmed | PASS — user confirmed |
| Core execution mean / p95 (ms) | 1.006 / 1.287 | 1.268 / 1.600 | 1.269 / 1.800 |
| Pixel conversion mean / p95 (ms) | 0.126 / 0.151 | 0.234 / 0.300 | 0.224 / 0.300 |
| Texture submission mean / p95 (ms) | 0.026 / 0.038 | 0.037 / 0.100 | 0.037 / 0.100 |
| Samples per timing series | 120 | 120 | 120 |
| Host sample rate (Hz) | 48000 | 48000 (running) | 48000 (running) |
| Current audio queue (ms) | 65.9 | 57.8 | 70.6 |
| Maximum audio queue (ms) | 71.2 | 70.9 | 74.8 |
| Audio queue capacity (ms) | 80 | 80 | 80 |
| Host underrun frames | 0 | 0 | 30 |
| Host overflow frames | 0 | 0 | 0 |
| Host output frames | 1142784 | 1437184 | 2640354 |
| Native device errors | 0 | Not applicable | Not applicable |
| Core PCM rate (Hz) | 32768 | 32768 | 32768 |
| Produced core PCM frames | 442913 | 1055018 | 1861656 |
| Core staging drops / empty FIFO | 0 / 0 | 0 / 0 | 0 / 0 |
| Executed instructions | 24422 | Not recorded | 106478 |
| GBA cycles | 226771912 | Not recorded | 953168025 |
| Execution state | Running | Not recorded | Running |
| Backup selection / detection / override | None / Unknown / None | None / Unknown / None | None / Unknown / None |
| Output device name | Not recorded | Not recorded | Not recorded |
| Browser version | Not applicable | Not recorded | Not recorded |
| Sustained unthrottled speed | Not measured | Not measured | Not measured |
| 30-minute queue/memory/latency measurement | Not measured | Not measured | Not measured |

All three reported core p95 values are below the plan's 12 ms target. Current
and maximum queue depths are within the planned 40–80 ms range. Brave has 30
host underrun frames; that counter is preserved even though the user confirmed
working playback. All host overflow, core staging-drop and empty-FIFO counters
are zero. These snapshots are not described as zero-loss Brave playback.

Capture durations, starting counters and lifecycle histories were not supplied,
so cumulative host output/underrun counters are not compared as rates or directly
against produced core frames. Manual acceptance is attributed to the user's
explicit confirmation; the snapshots alone do not establish a measured
30-minute latency/memory trend, physical playback latency, or sustained
unthrottled speed. Browser versions, output device names and separate live
44.1/96 kHz results remain unavailable. Reference hardware metadata at the top
of this document was not refreshed for these measurements. All earlier slice
measurements and the separate native probe evidence are preserved.

The supplied UI also showed the noise-scene status and its Z/X/Backspace help,
the all-channel mixing/Enable audio guidance, and unresolved-backup guidance.
Backup state is recorded above. The generic Load ROM/drop-file prompt and
arrow-key demo hint were visible in the native and Brave snapshots; these
interface messages are not additional Slice 19 measurements.

### Manual commands and acceptance

All manual checks are user-confirmed; these commands are retained for reproduction.

Native scene:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release
```

Chrome/Brave server:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in each browser. Click **Load noise scene**, then
**Enable audio**. Confirm the red/green square events have fading noise bursts
while pulse/wave/PCM continue. Press/release Z for an immediate alternate event,
X for short-mode noise and Backspace to isolate noise. Listen for chunk-boundary
clicks, unwanted aliasing, or accumulating delay. Check mute, pause/resume,
focus loss/return and reset. Run sustained mixing for 30 minutes, record starting
and ending queue/counter values, and supply app mean/p95 millisecond readings,
sample counts, browser version and device rate/name. Where available, repeat at
44.1/48/96 kHz; headless resampler proof alone does not imply live-device acceptance.

Recorded-signal and device reproduction:

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 roms/noise/build.py
python3 roms/noise/verify.py
cargo run --locked -p gba-tools --release -- verify-noise --capture /tmp/noise.wav
cargo run --locked -p gba-app --release -- --audio-probe noise
```

### Plan acceptance checklist

- [x] A visible scene event triggers noise with generation, envelope and length behavior.
- [x] Paired events exercise all four timer cascades, both FIFOs, DMA1/DMA2 refill and alternate timer selection.
- [x] Native/WASM recorded timing and sustained mixed signal pass at supported resampler rates without chunk discontinuities.
- [x] Actual native-device bounded playback probe passes with zero loss/error counters.
- [x] Manual native/Chrome/Brave scene, sound and lifecycle acceptance — user confirmed.
- [x] Manual mixed-playback continuity and latency acceptance at the reported 48 kHz rate — user confirmed; no audible aliasing/discontinuity reported.

Separate live 44.1/96 kHz runs and quantitative 30-minute measurements were not
recorded; automated resampler checks at all three rates remain passing.

Slice 19 is complete against the plan with automated checks passing and manual
native/Chrome/Brave acceptance explicitly confirmed by the user. Supplied
performance/audio snapshots are recorded above; unavailable measurements remain
marked as such.

## Slice 20 — supplied BIOS startup and Lycan UI: complete

Code adds exact 16 KiB BIOS validation, BIOS reset startup separate from controlled
diagnostics, guest SWI/IRQ execution through supplied firmware, protected fetch
latching and access-width lanes, CPU/DMA open-bus context, BIOS/backup retention
through reset, shared native/browser BIOS loading and bounded probe tooling.

Slice 20 is implemented, tested and measured. The user confirmed completion of
all testing and measurements on 2026-10-02. Manual results below are attributed
to that confirmation; numerical measurements and private-file identities were
not supplied for transcription.

Implementation verification on 2026-10-01:

- All 41 core/session tests pass, including focused BIOS, protected-read,
  save/reset and lifecycle checks.
- The mapped-to-unmapped block-load regression failed with zero instead of the
  prefetched opcode before its fix, then passed.
- BIOS installation during a paused save barrier failed before preserving pause,
  then passed. Invalid image length is rejected before state mutation.
- Earlier manifest guest fixtures and the pulse/wave/noise contracts pass;
  native/WASM checks, browser release build and strict Clippy pass.
- The BIOS diagnostic WASM contract compiles; supplied retail BIOS execution
  and diagnostic acceptance are complete — user confirmed on 2026-10-02.
- The native runner rejects a 4-byte BIOS and a synthetic reset-vector loop;
  the latter reaches its declared cycle limit without being mistaken for success.
- BIOS identity, five commercial ROM checkpoints and Linux/Chrome/Brave
  startup, input, sound, reset and save acceptance: tested and measured — user
  confirmed on 2026-10-02. Hashes, checkpoint values and raw logs are not recorded
  in this document.
- Native/browser core/conversion/upload mean and p95 timings: measured — user
  confirmed on 2026-10-02; numerical millisecond values are not recorded here.

The upstream BIOS diagnostic uses the shared contract in `roms/bios/contract.rs`.
Its 1908-byte ROM hash is
`9d7b369fa1aa661ff03692b3d79c6f644b623d72983d0fc890e6d87a0409a3c9`,
terminal PC is `0x080003c0`, success register is r12 = 0, and the frozen success
framebuffer hash is
`59ce42abae9825c2d2579c5cd838e47d88be917e37ea36ff162d46fc5d0991e3`.
Limits are 50000000 instructions and 168537600 cycles, including complete scanout.
Supplied-file execution and its measurements are complete — user confirmed on
2026-10-02. Actual runtime cycles/instructions/framebuffer hashes and private BIOS
identity values are not recorded here; the frozen values above describe the
contract, rather than a newly captured run.

### Lycan player UI and live pacing follow-up

- Clean player mode is the native/WASM default; `--debug-ui` and F1 retain all
  diagnostic controls. PCM auto-loads only at diagnostic startup.
- Live pacing bounds each callback to one frame of catch-up and 400000
  instructions. Work-limit exhaustion with cycle progress reports slowdown and
  drops backlog; zero-progress limits and core errors remain fatal.
- Native window configuration is centered, resizable, 760×540 by default and
  360×280 minimum. Native/web product titles and the visible toolbar identify Lycan.
- One compact toolbar contains ROM/save state, Audio menu, BIOS/ROM loading,
  Pause/Resume and Reset. The centered framebuffer continuously fits the viewport
  at 3:2 with nearest-neighbor filtering and a subtle dark frame.
- Existing save barriers, persistence and diagnostics remain intact.
- Automated follow-up checks passed: formatting, 32 core tests, 13 session tests,
  1 app test, strict workspace/all-targets Clippy, WASM check and Trunk release build.
  The sizing regression was RED with the previous 3× cap and GREEN after its removal.
- Native release play, continuous resizing, visible Lycan branding, F1 switching
  and uncluttered player presentation: tested and measured — user confirmed on
  2026-10-02. Sustained-play durations and numerical timing values are not recorded.

### Plan acceptance checklist

- [x] Supplied BIOS validation, reset startup, guest firmware execution and protected/open-bus access implemented and tested.
- [x] BIOS, cartridge backup and pause/save lifecycle retention implemented and tested.
- [x] Supplied BIOS diagnostic execution tested and measured — user confirmed.
- [x] Five commercial games and native/Chrome/Brave startup, input, audio, reset and saves tested and measured — user confirmed.
- [x] Native/browser core, conversion and upload performance measured — user confirmed; numerical values not recorded here.
- [x] Live work-budget recovery and zero-progress failure behavior tested.
- [x] Lycan window, branding, continuous scaling, compact controls and diagnostic switching implemented and tested — manual acceptance user confirmed.
- [x] Required automated native/WASM verification gates passed.

Manual commands and the historical five-game evidence table are in
[compatibility.md](compatibility.md). Its earlier pending entries predate the
2026-10-02 user confirmation recorded here. Slice 20 is complete: implemented,
tested and measured. No later slice implementation is included in this closeout.


## Slice 21 — affine background image: complete

Implemented and verified against `plan_final.md` section 21 on 2026-10-02.
The user confirmed all manual verification complete on 2026-10-02, including
Linux/Chrome/Brave acceptance. Chrome and Brave measurements supplied below
are recorded exactly for `affine-mode-5.gba`; existing timing entries above
are preserved. Slice 21 is complete.

The renderer now samples BG2/BG3 through signed integer affine coefficients,
signed 28-bit reference coordinates and per-line internal origins. Reference
writes reload the affected coordinate; PB/PD advance the internal coordinates;
the next frame reloads the MMIO reference values. Mode 1 combines text BG0/BG1
with affine BG2; mode 2 supports affine BG2/BG3. Modes 3/4 now use BG2 transforms,
and mode 5 clips to 160x128 with direct-color pages at 0 and 0xa000. Tiled affine
backgrounds honor wrapping and index-zero transparency. Layer composition retains
BG priority/tie order and normal objects. No affine-object implementation is included.

Controlled direct startup supplies identity matrices for earlier diagnostics;
supplied-BIOS startup clears them so firmware owns initialization. The existing
scanline drawing approximation remains: within-line rendering is not verified.

### Guest and capture evidence

Original source: [affine image assembly](../roms/affine/image.s).
Reproducible build and manual commands: [affine README](../roms/affine/README.md).
Frozen ROM identities and PC windows: [manifest](../roms/affine/manifest.toml).
Frozen full-frame PPM SHA-256 values: [capture identities](../roms/affine/captures.json).
The shared [geometry oracle](../roms/affine/contract.rs) checks source geometry
independently of renderer addressing and internal affine accumulators.

- Five ROM variants exercise mode-1 BG2, mode-2 BG3, and bitmap modes 3/4/5.
- Seven complete 240x160 captures per mode, at frames 20/24/28/32/36/40/44:
  35 captures, 1,344,000 pixel comparisons per platform, all pass natively and
  with the production WASM core in Node. Headless WASM is separate from browser UI acceptance.
- Four quarter-turn angles, 1x/2x source steps, centered reference origins,
  negative-coordinate clipping/wrapping, both mode-4/5 pages and mode-3 page-ignore behavior pass.
- Checkpoints require mailbox ID 0x00a1 at 0x03000000 plus angle/scale/page/wrap,
  the source-defined VCOUNT polling PC window and the completed-frame generation.
  The interactive guest has no terminal stop; a polling branch alone cannot pass.
- Each advance is limited to 2,000,000 instructions. The final target is
  12,359,424 cycles (44 frames), with at most 32 cycles of permitted instruction
  overshoot. Native observed completion evidence follows.

| Mode | ROM bytes | Final PC | Cycles | Instructions | Native/WASM captures |
|---|---:|---|---:|---:|---|
| 1 | 476 | `0x0800017c` | 12,359,430 | 1,031,600 | 7/7 pass each |
| 2 | 472 | `0x08000170` | 12,359,432 | 1,031,577 | 7/7 pass each |
| 3 | 480 | `0x0800018c` | 12,359,430 | 1,130,812 | 7/7 pass each |
| 4 | 504 | `0x08000198` | 12,359,424 | 1,169,504 | 7/7 pass each |
| 5 | 496 | `0x08000190` | 12,359,441 | 1,137,618 | 7/7 pass each |

Captures regenerate under `target/affine-captures/`; their 35 named identities
are frozen in `captures.json`. The native capture contact sheet was inspected;
that is headless image evidence, not native-window or browser acceptance.

### Focused RED/GREEN and regression gates

Only two focused core regressions were added in the existing test module:

- Mode-5 transformed page sampling/clipping: RED returned zero instead of
  0x1234 on the old renderer; GREEN samples the transformed alternate page
  and clips after the 128th source row.
- Mid-frame reference reload/line progression: RED returned row-1 color 992
  instead of reloaded row-0 color 31; GREEN also checks signed negative X
  clipping and the following source texel.

Verification passed: 48 workspace tests (33 core unit, 1 core backup integration,
13 session and 1 app), earlier manifest guest fixtures, pulse/wave/noise guest
contracts, workspace/all-targets Clippy with warnings denied, formatting,
native app check, WASM app check and Trunk release build. The affine verifier
also passes native full-frame captures, frozen capture hashes and the same
production-WASM oracle. Host: Linux x86_64; rustc 1.98.1
(48a229cea 2026-09-01). Browser versions and sustained UI speed are not recorded.

### Manual performance and platform acceptance

The user confirmed all manual checks complete on 2026-10-02: the five affine
ROMs, rotation/scaling/wrapping/clipping/page changes, input, pause/resume and
reset on Linux native, Chrome and Brave. This acceptance is attributed to the
user's confirmation; automated capture evidence remains separately documented.

The supplied browser snapshots both identify `affine-mode-5.gba`, Running,
angle 0°, 2x source step, page 0 and wrap=false. Each timing statistic has
120 samples. These timings apply to that ROM/state, rather than all five ROMs.

| Platform | Visual/input/pause/reset acceptance | Core mean/p95 ms | Conversion mean/p95 ms | Texture submission mean/p95 ms | Samples per metric | Sustained speed |
|---|---|---|---|---|---:|---|
| Linux native | complete — user confirmed | not recorded | not recorded | not recorded | not recorded | not recorded |
| Chrome | complete — user confirmed | 7.197 / 11.700 | 0.122 / 0.200 | 0.031 / 0.100 | 120 | not recorded |
| Brave | complete — user confirmed | 7.018 / 11.000 | 0.113 / 0.200 | 0.018 / 0.100 | 120 | not recorded |

| Browser snapshot | Instructions | GBA cycles | Core PCM rate | PCM produced | Staging drops | Empty FIFO |
|---|---:|---:|---|---:|---:|---:|
| Chrome | 32,171,501 | 384,334,071 | 32768 Hz | 750,652 | 0 | 0 |
| Brave | 24,229,591 | 289,142,008 | 32768 Hz | 564,730 | 0 | 0 |

Both snapshots report BIOS not loaded, audio off, Backup None, detected Unknown,
and no backup override. The diagnostic needs neither BIOS nor save hardware;
the generic unresolved-backup message does not indicate a failed save contract.
PCM counters describe core production while host playback was disabled.
Browser versions, native numerical timings, timings for modes 1–4 and a
sustained speed percentage were not supplied and remain not recorded. Their
absence does not change the user's explicit manual acceptance confirmation.

Commands retained for reproducing the completed checks:

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 roms/affine/build.py
python3 roms/affine/verify.py
cargo run --locked -p gba-app --release -- --debug-ui
```

Load `roms/affine-mode-1.gba` through `roms/affine-mode-5.gba` with Load ROM;
these original diagnostics do not need a BIOS. Resume if paused.

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in Chrome and Brave and repeat the same controls.

### Plan acceptance checklist

- [x] Buttons rotate/scale the original guest image through affine registers.
- [x] Affine sampling and reference-point behavior implemented; focused RED/GREEN evidence retained.
- [x] Modes 1/2 and bitmap transforms match complete expected captures in native and production WASM execution.
- [x] Mode-5 dimensions/page behavior verified through the same image contract.
- [x] Rotation, wrapping, clipping and page changes use integer arithmetic and match the source-geometry oracle.
- [x] Earlier guest regressions and required automated build/check gates pass.
- [x] Manual Linux, Chrome and Brave UI acceptance complete — user confirmed on 2026-10-02.
- [x] Supplied Chrome/Brave mode-5 millisecond measurements and 120-sample counts recorded exactly; unavailable measurements explicitly marked not recorded.

Slice 21 is complete: implemented, automatically verified and manually accepted
by the user on Linux, Chrome and Brave. Supplied browser performance evidence is
recorded above. Slice 22 was not started in this closeout.


## Slice 22 — affine objects: automated verification complete; manual acceptance pending

Implemented against `plan_final.md` section 22 on 2026-10-02. The user supplied Chrome and Brave running-session diagnostics and timing
measurements on 2026-10-02. Interactive visual acceptance and native UI checks
remain unconfirmed. Existing user-entered timings above are preserved.

The existing object sampler now decodes signed 8.8 PA/PB/PC/PD coefficients from
all 32 interleaved OAM matrix slots. It transforms around the center of normal
or doubled display bounds, floors negative fractional coordinates with arithmetic
shifts, clips against original source dimensions, and ignores regular flip bits
for affine objects. Source dimensions continue to determine 1D/2D tile stride;
expanded bounds never change addressing. Transparent color zero and the existing
OAM-order/BG-priority compositor are shared with regular objects. The renderer
retains its documented scanline approximation. Windows, mosaic and blending stay
outside this slice.

### Fixtures and acceptance evidence

Original guest and reproducible commands: [affine-object README](../roms/affine-object/README.md).
Frozen identities: [manifest](../roms/affine-object/manifest.toml).
Independent source-geometry oracle: [contract](../roms/affine-object/contract.rs).
Frozen capture hashes: [captures](../roms/affine-object/captures.json).

| Evidence | Result |
| --- | --- |
| Original interactive ROM | `affine-object.gba`, 16956 bytes; SHA-256 `48f4107eb9697d810aad537ec0fe94b2de7d736c1909444882bc3b028f463b6f` |
| Startup | Controlled direct ARM, no BIOS, no backup; app startup exception requires exact shipped bytes |
| Interactive completion rules | Continuous guest, polling PC `0x0800017c..=0x08000190`, mailbox ID `0x00a2`, seven exact control slots, completed framebuffer generation |
| Work bounds | 2,000,000 instructions per advance; at most 32 cycles of instruction-boundary overshoot; final target 72 frames / 20,224,512 cycles |
| RED before GREEN | Pre-feature renderer fails frame 20 at `(116,60)`: expected sprite `0x1405`, got BG `0x03e0`; `/tmp/gba-affine-object-red.txt` |
| Interactive native captures | PASS: 14 full-frame captures, 537,600 pixel comparisons; identity, 45/90/180-degree rotation, enlargement, normal/expanded bounds, color depths, tile layouts, transparent holes, BG priority, clipped top/left position |
| Final interactive state | Frame 72; PC `0x08000180`; cycles 20,224,513; instructions 1,683,561; mailbox state `[3,1,0,0,0,0,1]` |
| Applicable mGBA object comparison | PASS: standalone translation of `degenerateObjTransform`, six singular matrices, all 38,400 pixels against upstream BMP; full suite not executed |
| Upstream comparison identity | Revision `e6942030d25ffe3ba76c72b73a86da073ec857cc`; unmodified BMP SHA-256 `3bb3f88e4f6062c82ed263d88f990a04d5db514606a56186b8e606dea2ecfbc0`; MIT license retained |
| Standalone comparison ROM | `affine-object-degenerate.gba`, 404 bytes; SHA-256 `44b2cbb7bba8a13da6475c07e76e091297aa4f43409f8e958d40eb2975ab7fe1` |
| Standalone completion | Terminal PC `0x080000dc`, mailbox `0x00a3` / result 1; frame 4, cycles 1,123,588, instructions 57,266; 200,000-instruction bound |
| Production WASM contract | PASS: same 14 interactive captures and upstream comparison executed in Node |
| Capture artifacts | `target/affine-object-captures/frame-{4,20,24,28,32,36,40,44,48,52,56,60,64,68,72}.ppm`; independent geometry/BMP checks precede frozen SHA-256 comparisons |

The upstream comparison uses the
[mGBA object case](https://github.com/mgba-emu/suite/blob/e6942030d25ffe3ba76c72b73a86da073ec857cc/src/video.c#L106)
with its original palette, texels, object locations and matrices. Its libgba/menu
wrapper is replaced by standalone ARM startup. The pinned 256x128 expected image
is cropped to LCD width and its white backdrop extended to 160 rows; `verify.py`
checks this derivation against the retained raw BGR555 image. This comparison does
not claim mGBA runtime execution or completion of HBlank/window/effect tests.

### Automated gates and native core performance

| Check | Result |
| --- | --- |
| Core/session tests | PASS: 33 core unit tests, 1 backup integration test, 13 session tests |
| Workspace all-target Clippy | PASS with `-D warnings` |
| Formatting / whitespace | PASS: `cargo fmt --all -- --check`, `git diff --check`; the previous affine-background oracle received only rustfmt import ordering |
| Broad fixture runner | PASS; `/tmp/gba-affine-object-fixtures.txt` |
| Previous affine backgrounds | PASS: 35 native and production WASM captures in modes 1–5; `/tmp/gba-affine-object-backgrounds.txt` |
| Native release app | PASS: `cargo build --locked -p gba-app --release` |
| WASM app check | PASS: `cargo check --locked -p gba-app --target wasm32-unknown-unknown` |
| Release Trunk artifact | PASS with isolated absolute output `target/affine-object-web`; the first default-output attempt failed while writing the staged JS loader |
| Final object verifier | PASS: identity checks, independent BMP derivation, frozen capture hashes, native and production WASM contracts; `/tmp/gba-affine-object-green.txt` |
| Native steady core benchmark | 600 frames after frame-20 initialization; mean **2.663 ms**, p95 **2.752 ms**; stable final image/mailbox; `/tmp/gba-affine-object-bench.txt` |

Reproduce the isolated browser release artifact with:

```bash
env -u NO_COLOR trunk --config web/Trunk.toml build --release --dist /home/aditya/Projects/GBA/gba-rs/target/affine-object-web
```

Environment: Linux 7.2.7-arch1-1, Intel Core i7-13620H, Rust 1.98.1;
Trunk 0.21.14. Governor, thermals, display refresh and audio device were not
recorded. Native headless timing excludes conversion, texture submission and UI.

### User-owned manual measurements

| Platform | Manual behavior | Core mean/p95 ms | Conversion mean/p95 ms | Texture submission mean/p95 ms | Sustained speed | Version |
| --- | --- | --- | --- | --- | --- | --- |
| Linux native | Not verified | Not measured | Not measured | Not measured | Not measured | Not recorded |
| Chrome | Running snapshot recorded; full behavior not verified | 7.553 / 11.200 | 0.117 / 0.200 | 0.023 / 0.100 | Not measured | Not recorded |
| Brave | Running snapshot recorded; full behavior not verified | 8.242 / 16.100 | 0.135 / 0.300 | 0.032 / 0.100 | Not measured | Not recorded |


Both browser snapshots identify `affine-object.gba` as loaded and Running. Each
core, conversion and texture-submission timing summary contains **120 samples**.
The values above are copied exactly from the user's diagnostics; they are app
measurements and are separate from the native headless benchmark. Browser versions,
refresh rates, governor/thermal conditions, elapsed wall time and sustained
emulation speed were not supplied. A Running snapshot does not establish the
complete rotation, bounds, transparency, priority or lifecycle checklist.

| Diagnostic | Chrome | Brave |
| --- | --- | --- |
| Executed instructions | 29,723,808 | 17,676,377 |
| GBA cycles | 356,636,433 | 212,088,335 |
| Core PCM rate | 32,768 Hz | 32,768 Hz |
| PCM samples produced | 696,555 | 414,235 |
| PCM staging drops | 0 | 0 |
| Empty FIFO count | 0 | 0 |
| Host audio | Off; not manually verified | Off; not manually verified |
| Cartridge backup | None; detected Unknown; override None | None; detected Unknown; override None |
| BIOS loaded | 16,384 bytes | 16,384 bytes |

Both snapshots report BIOS SHA-256
`fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`.
The original diagnostic still uses its controlled startup route; the presence of
loaded BIOS bytes does not establish BIOS boot for this ROM. Backup None is
expected for this guest, which has no save hardware. The generic backup-override
notice does not require selecting a save protocol for it. PCM counters record
core generation only; host audio was disabled.

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release -- --debug-ui
```

Load `roms/affine-object.gba`, then Resume. Right rotates, Up enlarges, Z toggles
expanded bounds, X toggles color depth, S toggles tile layout, A toggles BG
priority, and Left moves the sprite across the top/left clipping boundary. Release
each key between presses. X/S preserve the image. Transparent holes show green;
A hides/restores the sprite. Verify pause/resume/reset, input release on focus
loss and crisp resizing. Reset restores frame 20. F1 shows timing summaries.

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --dist /home/aditya/Projects/GBA/gba-rs/target/affine-object-web --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in Chrome and Brave, load the same ROM, and repeat.
The README gives the exact capture replay sequence. Enter only observed timing
values and confirm manual acceptance separately.

### Comparison against plan section 22

- [x] Add affine object parameters, normal/expanded bounds and source addressing.
- [x] Exercise normal and expanded bounds with fixed full-frame captures.
- [x] Compare the applicable mGBA singular-matrix object case against its upstream image.
- [x] Automatically verify transformed pixels, transparency and BG priorities at recorded positions on native and production WASM.
- [x] Record user-supplied Chrome/Brave app timings and running-session diagnostics.
- [x] User-confirmed Linux/Chrome/Brave runtime behavior and performance.
- [ ] Record browser versions and sustained speed.

The existing user-marked manual-acceptance checkbox above is preserved. The
supplied snapshots independently establish running sessions and timing evidence;
they do not newly confirm every visual/lifecycle check. The pending manual-status
text is retained until that scope is explicitly confirmed. Browser versions and
sustained speed remain unrecorded. No later slice was implemented.

## Slice 23 — window masks and mosaic: automated correction verified; original manual acceptance recorded

Implemented against `plan_final.md` section 23 on 2026-10-02. Existing timing
entries are preserved. On 2026-10-02 the user confirmed all manual Slice 23
checks, including Linux application and Chrome/Brave visual/lifecycle acceptance.
The supplied browser diagnostics and timings are recorded below. Instructions and exact capture replay:
[window README](../roms/window/README.md).

The compositor now selects WIN0, WIN1, OBJWIN or WINOUT before background and
object priority decisions. Rectangular bounds are half-open, support wrapping,
and are empty when start equals end. WIN0 wins overlap, then WIN1, then opaque
object-window texels; transparent object-window texels preserve the outside mask.
Bits 0–4 gate individual layers while backdrop remains available. Bit 5 retains
color-effect permission for section 24; alpha/brightness operations are not part
of this slice. BG mosaic samples screen-coordinate groups before scroll/affine
addressing; OBJ mosaic samples screen-aligned groups, clamped at the object origin,
before flips/transforms. Horizontal coverage includes the final partial mosaic block.
Rendering retains the existing scanline approximation.

### Fixture and acceptance evidence

The original MIT guest, [manifest](../roms/window/manifest.toml), independent
[geometry oracle](../roms/window/contract.rs) and frozen
[capture hashes](../roms/window/captures.json) are reproducible with:

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 roms/window/build.py
python3 roms/window/verify.py
cargo run --locked -p gba-tools --release -- bench-window
```

| Evidence | Result |
| --- | --- |
| ROM | `window.gba`, 632 bytes; SHA-256 `067e7b896ccb2cf9a763b4dc685527fd6a470c41bf8440c0dfe4f467f98c9920` |
| Startup | Controlled direct ARM, no BIOS, no backup; app exception requires exact shipped bytes |
| Completion | Continuous guest; polling PC `0x080001b0..=0x080001c4`, mailbox `0x03000000` ID `0x00a4` plus four exact control slots, completed framebuffer generation |
| Execution bounds | 2,000,000 instructions per advance; at most 32 cycles of instruction-boundary overshoot |
| Meaningful RED | Before implementation, outside pixel `(0,0)` incorrectly rendered BG0 `0x001f` instead of allowed BG1 `0x03e0`; final guest also fails against isolated unchanged HEAD renderer |
| Native GREEN | 11 full-frame captures at frames 12,16,20,24,28,32,36,40,44,48,52; 422,400 pixel comparisons |
| Captured behavior | Moving WIN0, visible layer toggle, WIN0/WIN1 overlap, object-window opaque/transparent texels, preserved outside BG/OBJ, wrapped/empty horizontal bounds, horizontal/vertical 4x4 BG and 3x3 OBJ mosaic |
| Final guest state | Frame 52; PC `0x080001b4`; cycles 14,606,592; instructions 1,217,728; mailbox controls `[32,0,0,2]` |
| Production WASM GREEN | Same 11 guest captures and independent image oracle executed in Node |
| Capture artifacts | `target/window-captures/frame-{12,16,20,24,28,32,36,40,44,48,52}.ppm`; each image checked against geometry before frozen SHA-256 |
| Core/session tests | PASS: 34 core unit tests, 1 backup integration test, 13 session tests; one focused unaligned OBJ mosaic regression |
| Regression captures | PASS: 35 affine-background native/WASM captures; 14 affine-object native/WASM captures and existing standalone mGBA singular-matrix comparison |
| Broad fixtures | PASS: `cargo run --locked -p gba-tools --release -- fixtures run` |
| Static gates | PASS: all-target workspace Clippy with `-D warnings`, formatting, `git diff --check` |
| Application builds | PASS: native release app, WASM app check, isolated release Trunk output `target/window-web` |
| Retained evidence | `target/window-evidence/{results,baseline-red,verify,tests,clippy,native-build,wasm-check,object-regression,background-regression,fixtures,bench,trunk,format,diff}.txt` |

The new replay catches real layer leakage, incorrect overlap precedence, opaque
object-window holes and incorrect mosaic sampling; previous fixtures do not
exercise these behaviors. The focused renderer regression separately covers the
unaligned right edge at X=101 and screen-aligned vertical sampling at Y=61.
Affine mosaic and vertically wrapped window bounds are implemented through the
shared samplers/range selection but are not independently captured by this focused
guest. Color-effect permission is retained, while its visible effect will be
verified when section 24 introduces blending. No mGBA window comparison is claimed.

### Performance and manual acceptance

| Platform | Behavior | Core mean/p95 ms | Conversion mean/p95 ms | Texture submission mean/p95 ms | Sustained speed | Version |
| --- | --- | --- | --- | --- | --- | --- |
| Native headless | 600 stable frames after frame-12 initialization; image/mailbox checked | 2.286 / 2.282 | Not applicable | Not applicable | Not measured | Rust 1.98.1 |
| Linux native UI | User-confirmed PASS | Not measured | Not measured | Not measured | Not measured | Not recorded |
| Chrome | User-confirmed PASS | 7.447 / 11.100 | 0.111 / 0.200 | 0.015 / 0.100 | Not measured | Not recorded |
| Brave | User-confirmed PASS | 7.386 / 11.400 | 0.106 / 0.200 | 0.023 / 0.100 | Not measured | Not recorded |

### User-recorded Chrome / Brave diagnostics

Both snapshots identify `window.gba` as loaded and **Running**. Each core,
pixel-conversion and texture-submission summary contains **120 samples**. Timings
are preserved exactly as supplied and remain separate from the native headless
benchmark. The user explicitly confirmed all manual checks: window movement,
layer changes, rectangular overlap, object-window transparency, mosaic,
wrapped/empty bounds, preserved outside layers, pause/resume/reset, focus-loss
key release and resizing. Manual acceptance is attributed to that confirmation,
not inferred from the Running snapshots.

| Diagnostic | Chrome | Brave |
| --- | --- | --- |
| Executed instructions | 39,100,392 | 52,257,824 |
| GBA cycles | 469,128,406 | 626,994,556 |
| Core PCM rate | 32,768 Hz | 32,768 Hz |
| PCM samples produced | 916,266 | 1,224,598 |
| PCM staging drops | 0 | 0 |
| Empty FIFO count | 0 | 0 |
| Host audio | Off | Off |
| Cartridge backup | None; detected Unknown; override None | None; detected Unknown; override None |
| BIOS loaded | 16,384 bytes | 16,384 bytes |

Both snapshots report BIOS SHA-256
`fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`.
This diagnostic uses controlled ARM startup; loaded BIOS bytes do not establish
BIOS boot for this ROM. Backup None is expected because this guest has no save
hardware. PCM counters establish core sample generation; the supplied snapshots
show host audio off. Browser versions, elapsed wall time, sustained emulation
speed, refresh rate and native UI numerical timings were not supplied and remain
unrecorded/unmeasured. Chrome and Brave core mean/p95 values are below the
16.74 ms frame budget; the snapshots do not independently measure sustained speed.

Environment: Linux 7.2.7-arch1-1, Intel Core i7-13620H, Rust 1.98.1,
Trunk 0.21.14. Governor, thermals, refresh rate and audio device were not recorded.
The retained headless run excludes conversion, texture submission, UI and host
audio. Its core mean/p95 are below the 16.74 ms frame budget; this does not prove
browser speed. An earlier run measured 2.504 / 2.692 ms; the table uses the final
retained run rather than mixing samples.

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release -- --debug-ui
```

Load `roms/window.gba`, then Resume. Right/Left move WIN0, Z changes its permitted
layers, X toggles mosaic, Up selects wrapped WIN0, Down selects empty WIN0.
Release between presses. Replay Right, Z, X, Right, Right, Left, X, Z, Up, Down
and compare the frozen captures. Reset restores the initial scene. Verify overlap,
object-window holes, unchanged outside layers, pause/resume/reset, focus-loss key
release and crisp resizing. F1 exposes timing summaries.

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --dist /home/aditya/Projects/GBA/gba-rs/target/window-web --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in Chrome and Brave and repeat the checks. Record only
observed timing values, browser versions and sustained speed on future runs.
The checks above are user-confirmed complete; commands are retained for reproduction.

### Comparison against plan section 23

- [x] Buttons move a window and change visible layers through guest register writes.
- [x] Add window bounds and per-layer/effect permission masks.
- [x] Add an object-window case and a focused mosaic case.
- [x] Capture rectangular boundaries, overlap, wrapped/empty bounds and transparent object-window pixels.
- [x] Automatically verify expected masked regions and preserved outside layers on native and production WASM.
- [x] Record evidence-backed performance and reproduction commands without changing prior user timings.
- [x] User-confirmed Linux application visual/lifecycle acceptance.
- [x] User-confirmed Chrome/Brave visual/lifecycle acceptance and supplied browser measurements.

The original Section 23 closeout was implemented, automatically verified and manually
accepted by the user on Linux, Chrome and Brave. The mosaic correction below has
separate automated evidence; its changed guest awaits manual replay. Missing browser versions and numerical
sustained-speed/native UI measurements are explicitly recorded above. Section 24
was not started.

### OBJ mosaic correctness follow-up (2026-10-02)

The previous oracle incorrectly grouped OBJ samples relative to its local origin,
and Y=60 accidentally aligned with the three-pixel screen grid. The corrected
oracle uses `local_x.saturating_sub(x % 3)` and
`local_y.saturating_sub(y % 3)` only inside the visible object's geometric bounds.
The guest now starts its ordinary object at Y=61. It retains the original mailbox,
polling PC, instruction limits and frame checkpoints.

Before changing coverage, the focused regression
`unaligned_object_mosaic_repeats_through_right_edge` failed on line 61: pixels
109–110 were absent for an eight-pixel object at X=101 with three-pixel mosaic.
Its source rows distinguish the vertical rule at Y=61: rows 61/62 use source row
0, row 63 uses source row 2. A shared `horizontal_coverage` helper now rounds the
exclusive right edge to the next screen mosaic boundary for visible OBJ and
OBJWIN paths; source dimensions and vertical clipping retain nominal bounds.

Expected right-edge behavior was compared to the rounding and repeat logic in
[mGBA's software OBJ renderer](https://github.com/mgba-emu/mgba/blob/master/src/gba/renderers/software-obj.c).
This is a source-behavior comparison, not a run of an installed mGBA emulator.
Evidence: `target/window-evidence/mosaic-red.txt` and `mosaic-green.txt`.

All 11 corrected geometry captures passed **before** `captures.json` was
regenerated (`target/window-evidence/corrected-oracle.txt`). Frozen-image checks
and the same production WASM geometry replay then passed. The new ROM is still
632 bytes; its current identity is recorded in the fixture table and manifest.
The original browser timings and manual confirmation above are preserved as
historical evidence for the earlier ROM (`3351328bb7580a9b246d87d7f04226b23833e29a87e60c59609e9a035102737a`).
Manual visual acceptance of the corrected Y=61 guest has not been repeated;
load the rebuilt `roms/window.gba` and replay the README sequence to verify it.
No updated browser timings are inferred from the earlier snapshots.

Correction gates: PASS for 34 core unit tests, one backup integration test, 13
session tests, workspace all-target Clippy with warnings denied, 11 corrected
native/WASM window captures, 14 prior affine-object captures plus the existing
mGBA singular-matrix comparison, 35 prior affine-background captures, broad
fixtures, native release app, WASM app check, release Trunk artifact, formatting
and whitespace checks. Logs: `target/window-evidence/mosaic-results.txt` and
`mosaic-{verify,tests,clippy,affine-object,affine-bg,fixtures,native,wasm,trunk,format,diff}.txt`.
The first correction Clippy run flagged the test's OAM chunk iteration; switching
to typed eight-byte chunks resolved it and the final gate passed. Existing
headless timings also describe the earlier build; no new performance measurements
were fabricated for this correction.

## Slice 24 — alpha blending, brightness and target selection: automated verification complete; runtime diagnostics recorded

Implemented against `plan_final.md` section 24. The renderer now retains the two
nearest visible surfaces after window filtering and priority resolution, then
applies BLDCNT target selection, BLDALPHA blending, or BLDY brightness at the
scanline drawing boundary. Alpha uses only the immediately lower visible
surface. Semi-transparent OBJs enter the color-effects stage as the top OBJ;
they force alpha over an eligible second target before the ordinary window SFX
gate. A later OBJ cannot blend through the top OBJ. Raw RGB555 bit 15 remains
available to blend arithmetic and is discarded at final composition.

### Guest contract and automated evidence

The original [blend guest](../roms/blend/README.md), [frozen manifest](../roms/blend/manifest.toml),
and independent [framebuffer oracle](../roms/blend/contract.rs) build and verify
with:

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 roms/blend/build.py
python3 roms/blend/verify.py
cargo run --locked -p gba-tools --release -- bench-blend
```

| Evidence | Result |
| --- | --- |
| ROM identity | `blend.gba`, 612 bytes; SHA-256 `525bdb984a951fc7a757585a363599d87c7eb022fc802577fb81727023eff2d8` |
| Startup | Controlled ARM, no BIOS and no backup; app uses the controlled path only for the exact shipped ROM bytes |
| Completion | Interactive guest; code PC window `0x08000000..0x08000400`; mailbox `0x03000000` ID `0x00a5`, state 7 at the final capture |
| Execution bounds | At most 2,000,000 instructions per advance; at most 32 cycles of instruction-boundary overshoot |
| Meaningful RED | Against unchanged HEAD, state 0 passed; state 1 failed at frame 16 pixel `(0,0)`: old output `0x001f`, expected alpha result `0x4010` |
| Native GREEN | 8 full-frame captures, frames 12–40; 307,200 exact pixel comparisons against the independent oracle |
| Production WASM GREEN | The same 8 guest captures and 307,200 pixel comparisons passed in Node |
| Final guest state | Frame 40; PC `0x080001c8`; cycles 11,235,846 (6-cycle overshoot); 936,866 instructions; mailbox state 7 |
| Captured behavior | BG0/BG1 alpha; brighten/darken; coefficient saturation; transparent OBJ texels; semitransparent OBJ forced alpha; immediate lower target; WIN0 layer and SFX masks |
| Native captures | `target/blend-captures/frame-{12,16,20,24,28,32,36,40}.ppm` |
| Focused core regressions | PASS: immediate visible second target; semitransparent OBJ forced-alpha ordering and window gate; rounded arithmetic and hidden green precision |
| Prior display regressions | PASS: 11 window captures, 14 affine-object captures including its mGBA singular-matrix comparison, and 35 affine captures across modes 1–5 |
| Broad fixture runner | PASS: `cargo run --locked -p gba-tools --release -- fixtures run` |
| Workspace tests | PASS: 37 core tests, 1 backup integration test, 13 session tests, and 1 app test |
| Static gates | PASS: all-target workspace Clippy with `-D warnings`, formatting, and `git diff --check` |
| Application builds | PASS: native release app, WASM app check, production release Trunk build |

The pinned mGBA suite revision `e6942030d25ffe3ba76c72b73a86da073ec857cc`
contains no standalone blend ROM. The applicable hardware-confirmed behavior
from [mGBA issue #3804](https://github.com/mgba-emu/mgba/issues/3804) is
reproduced in state 4: a semi-transparent OBJ overlapping a valid second target
uses BLDALPHA even when BLDCNT requests brighten or darken. The issue attachment
was not copied or run; its redistribution terms were not established. This is a
source-behavior comparison, not a direct mGBA execution.

### Performance and user-supplied runtime diagnostics

The 2026-10-02 native, Chrome, and Brave diagnostics below were supplied by the
user. All three report `blend.gba` loaded and the app `Running`.

| Platform | Core mean/p95 ms | Pixel conversion mean/p95 ms | Texture submission mean/p95 ms | Samples (core / conversion / texture) | Sustained speed |
| --- | --- | --- | --- | --- | --- |
| Native headless | 2.549 / 2.569 | Not applicable | Not applicable | 600-frame benchmark; per-stage count not emitted | Not measured |
| Linux native UI | 8.215 / 10.696 | 0.077 / 0.101 | 0.019 / 0.027 | 120 / 120 / 120 | Not measured |
| Chrome | 6.732 / 8.300 | 0.079 / 0.200 | 0.024 / 0.100 | 120 / 120 / 12 | Not measured |
| Brave | 7.716 / 12.400 | 0.109 / 0.200 | 0.019 / 0.100 | 120 / 120 / 120 | Not measured |

### Runtime snapshots supplied by the user

| Diagnostic | Linux native UI | Chrome | Brave |
| --- | ---: | ---: | ---: |
| Loaded ROM / status | `blend.gba` / Running | `blend.gba` / Running | `blend.gba` / Running |
| Executed instructions | 54,102,041 | 26,450,459 | 15,158,307 |
| GBA cycles | 649,090,086 | 317,337,835 | 181,862,765 |
| BIOS | Not loaded | Not loaded | Not loaded |
| Core PCM rate | 32,768 Hz | 32,768 Hz | 32,768 Hz |
| PCM samples produced | 1,267,754 | 619,800 | 355,200 |
| PCM staging drops | 0 | 0 | 0 |
| Empty FIFO count | 0 | 0 | 0 |
| Host audio | Off | Off | Off |
| Backup / detected / override | None / Unknown / None | None / Unknown / None | None / Unknown / None |

The reported core p95 values are below the 16.74 ms frame budget. These are
per-stage summaries and do not establish sustained speed or end-to-end frame
time. The snapshots establish that the ROM was loaded and running; they do not
identify which guest state was displayed or explicitly confirm the visual
transition and overlap checks. State-by-state visual acceptance remains
unconfirmed. Browser versions, display refresh, and sustained wall time were
not supplied. For the manual sequence and launch commands, use the [blend
README](../roms/blend/README.md).

Environment for automated measurements: Linux 7.2.7-arch1-1, Intel Core i7-13620H, Rust 1.98.1, Node 26.10.0, Trunk 0.21.14. Governor, thermals, display refresh, audio device, and browser versions were not recorded.

## Slice 25 — timed HBlank DMA raster effect: automated and manual acceptance complete

Implemented against `plan_final.md` section 25 and `slice25.md`. DMA0 now
latches `DMA0CNT_H`, channel-specific address and count masks are applied, and
HBlank/VBlank requests activate all eligible DMA channels together. HALT wakes
for the next visible HBlank when any ordinary HBlank DMA is armed. Repeated
HBlank requests reload the count and destination as configured while the source
continues through its table. Transfers remain beat-based, so the existing
DMA0-to-DMA3 channel order resolves simultaneous requests. HBlank requests in
VBlank do not consume raster entries.

### Guest contract and automated evidence

The original [raster guest](../roms/raster/README.md),
[frozen manifest](../roms/raster/manifest.toml), and
[independent framebuffer oracle](../roms/raster/contract.rs) build and verify
with:

```bash
cd /home/aditya/Projects/GBA/gba-rs
python3 roms/raster/build.py
python3 roms/raster/verify.py
cargo run --locked -p gba-tools --release -- bench-raster
```

| Evidence | Result |
| --- | --- |
| ROM identity | `raster.gba`, 1,880 bytes; SHA-256 `6955629784fedcb3a5157dc0640339bdd8e9270bf3c582b78e2865fbc5d0e68d` |
| Startup | Controlled ARM, no BIOS and no backup; the app uses the controlled path for the exact shipped diagnostic ROM |
| Completion | Interactive guest; ROM PC window `0x08000000..0x08001000`; mailbox `0x03000000`, ID `0x00a6` |
| Execution bounds | At most 2,000,000 instructions per advance and 32 cycles of instruction-boundary overshoot |
| Meaningful RED | Before core changes, frame 12 failed because DMA IF was `0x0000` instead of DMA0's expected `0x0100`; log: `target/raster-evidence/verify-red.txt` |
| Native GREEN | Six complete frames; 230,400 exact pixel comparisons; frames 12, 13, 17, 18, 22, and 23 |
| Production WASM GREEN | The same six guest captures and 230,400 pixel comparisons passed in Node |
| Final guest state | Frame 23; PC `0x0800018c`; cycles 6,460,624; 535,829 instructions; state 2; IF snapshot `0x0900` |
| Raster behavior | State 0: DMA0 increasing brightness; state 1: DMA3 decreasing brightness; state 2: both channels complete, DMA0 runs first, and DMA3 supplies the final values matching state 1 |
| Line ownership | The independently calculated full-frame image verifies each HBlank table entry affects the following visible scanline; VBlank source progression is separately covered by the core regression |
| Native captures | `target/raster-captures/frame-{12,13,17,18,22,23}.ppm` |
| Focused core regressions | PASS: 160 visible HBlank repeats and VBlank exclusion; simultaneous DMA0/DMA3 priority; DMA0 14-bit count and 27-bit address latching |
| Prior display regressions | PASS: blend (8 frames), window (11), affine object (14 plus singular matrices), and affine backgrounds (35 across modes 1–5), including their production-WASM contracts |
| DMA/audio compatibility | PASS: WASM cartridge DMA/PCM route; pulse, wave, and noise contracts; noise DMA1/DMA2 FIFO refills and chunk invariance |
| Broad fixture runner | PASS: `cargo run --locked -p gba-tools --release -- fixtures run` |
| Workspace tests | PASS: 40 core tests, 1 backup integration test, 13 session tests, and 1 app test |
| Static gates | PASS: workspace all-target Clippy with `-D warnings`, formatting, and `git diff --check` |
| CI workflow syntax | Not parser-validated: PyYAML, actionlint, and Ruby are unavailable in this environment |
| Application builds | PASS: native release build, WASM app check, and production Trunk release build |
| Release benchmark | `bench-raster` completed its 600-frame headless run; millisecond timing fields are reserved for manual entry below |

The implementation stays scanline-granular. The fixture demonstrates a
one-line HBlank reproduction, so no within-line renderer changes were needed.
The automated audio/PCM regressions passed; host playback was not manually
assessed.

### Performance and manual visual acceptance

On 2026-10-02, the user supplied the Linux native, Chrome, and Brave snapshots
below and confirmed all manual checks were complete. The user confirmed all
three raster states, expected DMA IF progression (`0x0100`, `0x0800`,
`0x0900`), and the state-2 match with state 1. The supplied state-2 snapshots
show IF `0x0900`. Browser versions, display refresh, and sustained wall time
were not supplied. The host audio output was off in each snapshot, so audible
playback was not manually assessed.

| Platform | Core mean/p95 ms | Pixel conversion mean/p95 ms | Texture submission mean/p95 ms | Samples (core / conversion / texture) | Visual acceptance |
| --- | --- | --- | --- | --- | --- |
| Native headless | User entry pending | Not applicable | Not applicable | 600-frame release benchmark completed | Not applicable |
| Linux native UI | 7.509 / 9.875 | 0.072 / 0.099 | 0.016 / 0.025 | 120 / 120 / 120 | All states checked; user confirmed |
| Chrome | 7.680 / 11.800 | 0.097 / 0.200 | 0.019 / 0.100 | 120 / 120 / 12 | All states checked; user confirmed |
| Brave | 7.908 / 12.100 | 0.114 / 0.200 | 0.023 / 0.100 | 120 / 120 / 120 | All states checked; user confirmed |

The core p95 values are below the 16.74 ms nominal GBA frame period. These
per-stage summaries do not establish end-to-end frame time or sustained speed.

### Runtime snapshots supplied by the user

| Diagnostic | Linux native UI | Chrome | Brave |
| --- | ---: | ---: | ---: |
| Loaded ROM / status | `raster.gba` / Running | `raster.gba` / Running | `raster.gba` / Running |
| Raster state / DMA IF snapshot | State 2 / `0x0900` | State 2 / `0x0900` | State 2 / `0x0900` |
| Executed instructions | 50113088 | 12303125 | 15436136 |
| GBA cycles | 604537362 | 148635132 | 186354032 |
| BIOS | Not loaded | Not loaded | Not loaded |
| Core PCM rate | 32768 Hz | 32768 Hz | 32768 Hz |
| PCM samples produced | 1180737 | 290302 | 363972 |
| PCM staging drops | 0 | 0 | 0 |
| Empty FIFO count | 0 | 0 | 0 |
| Host audio | Off | Off | Off |
| Backup / detected / override | None / Unknown / None | None / Unknown / None | None / Unknown / None |

For reproduction, launch the native app from the repository root and load
`roms/raster.gba` through the debug UI's **Load ROM** picker:

```bash
cd /home/aditya/Projects/GBA/gba-rs
cargo run --locked -p gba-app --release -- --debug-ui
```

For Chrome and Brave, start the production web app and load the same ROM using
the browser's **Load ROM** picker:

```bash
cd /home/aditya/Projects/GBA/gba-rs
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in each browser. The manual sequence uses Right
to cycle through states 0, 1, and 2 and Left to go backward. It checks the
gradients, matching state-1/state-2 images, and the DMA IF diagnostic line.

Automated build environment: Linux 7.2.7-arch1-1, Intel Core i7-13620H,
Rust 1.98.1, Node 26.10.0, and Trunk 0.21.14. Governor, thermals, display
refresh, audio device, and browser versions were not recorded.


## Slice 26 — Pokémon FireRed: complete by user-declared manual acceptance

On 2026-10-02, the user supplied the Linux native, Chrome, and Brave FireRed
snapshots below and explicitly requested that Slice 26 be marked done. Slice 26
is recorded as complete on that user-declared acceptance. The diagnostics and
remaining measured limitations are retained separately; completion does not
mean that all three platforms recorded zero audio underruns.

### Cartridge and BIOS identity

The local cartridge and BIOS identities were checked when recording this entry.
The BIOS hash matches all three supplied runtime snapshots.

| Field | Recorded value |
| --- | --- |
| Cartridge | `local-roms/firered.gba` |
| ROM size | 16,777,216 bytes |
| ROM SHA-256 | `3d0c79f1627022e18765766f6cb5ea067f6b5bf7dca115552189ad65a5c3a8ac` |
| Header title / game code | `POKEMON FIRE` / `BPRE` |
| Region marker / revision | Header region marker `E` (from `BPRE`) / revision 0 |
| BIOS | User-supplied firmware; hardware reset-vector boot |
| BIOS size | 16,384 bytes |
| BIOS SHA-256 | `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570` |
| Cartridge backup | `Some(Flash128)`; detected `Identified(Flash128)`; override `None` |

### User-supplied live performance

All timing values are copied exactly from the user's latest snapshots. Each
metric contains 120 samples. The three snapshots represent separate sessions
and different backup states; they are not a controlled replay of identical
save states, inputs, or gameplay scenes.

| Platform | Core mean/p95 ms | Pixel conversion mean/p95 ms | Texture submission mean/p95 ms | Samples (core / conversion / texture) |
| --- | --- | --- | --- | --- |
| Linux native UI | 7.946 / 12.864 | 0.042 / 0.073 | 0.001 / 0.002 | 120 / 120 / 120 |
| Chrome | 8.182 / 13.700 | 0.057 / 0.100 | 0.000 / 0.000 | 120 / 120 / 120 |
| Brave | 8.691 / 14.100 | 0.073 / 0.200 | 0.004 / 0.000 | 120 / 120 / 120 |

Brave's texture mean/p95 pair is retained exactly as supplied; the apparent
ordering is not corrected or replaced with an inferred value. All three core
p95 values are below the nominal 16.7427 ms GBA frame period, but exceed the
plan's less-than-12 ms core p95 goal. These individual stage summaries do not
establish end-to-end frame time or sustained wall-clock emulation speed.

### Runtime snapshots supplied by the user

| Diagnostic | Linux native UI | Chrome | Brave |
| --- | ---: | ---: | ---: |
| ROM / execution state | `firered.gba` / Running | `firered.gba` / Running | `firered.gba` / Running |
| ROM action/status text | `Reset firered.gba` | `Loaded firered.gba` | `Loaded firered.gba` |
| Backup restoration status | Backup restored | No stored backup; new cartridge | No stored backup; new cartridge |
| Executed instructions | 1970298206 | 1067216308 | 1097262556 |
| GBA cycles | 5677997955 | 3124420681 | 3209665227 |
| Host output rate | 48000 Hz | 48000 Hz | 48000 Hz |
| Browser AudioContext state | Not applicable | running | running |
| Current queue depth | 45.9 ms | 49.9 ms | 33.8 ms |
| Maximum queue depth | 75.6 ms | 57.5 ms | 73.5 ms |
| Queue capacity | 80 ms | 80 ms | 80 ms |
| Underrun events | 12 | 115 | 12065 |
| Underrun frames | 3503 | 146 | 15440 |
| Overflow events | 0 | 0 | 0 |
| Overflow frames | 0 | 0 | 0 |
| Audio callbacks | 158622 | 69752 | 83624 |
| Maximum callback size | 512 frames | 128 frames | 128 frames |
| Host output frames | 16292945 | 8805083 | 9045124 |
| Native device errors | 0 | Not reported by browser adapter | Not reported by browser adapter |
| Core PCM rate | 32768 Hz | 32768 Hz | 32768 Hz |
| Core PCM frames produced | 11089839 | 6102384 | 6268877 |
| PCM staging drops | 0 | 0 | 0 |
| Empty FIFO count | 14 | 14 | 14 |
| Backup / detection / override | `Some(Flash128)` / `Identified(Flash128)` / `None` | `Some(Flash128)` / `Identified(Flash128)` / `None` | `Some(Flash128)` / `Identified(Flash128)` / `None` |
| Live pacing indicator | Not included in supplied snapshot | Not included in supplied snapshot | Not included in supplied snapshot |

Underrun events count audio callbacks with missing output frames, rather than
individual audible interruptions. At 48 kHz, the reported missing frames
represent approximately 0.072979 seconds of aggregate missing audio on native,
0.304771 seconds on Chrome, and 32.167917 seconds on Brave. These counters are
cumulative and do not establish when the missing audio occurred. Brave's
substantial underrun count remains a recorded audio limitation despite its
core p95 fitting within one GBA frame. Zero overflows and zero core staging
drops are recorded on all three platforms.

### Implementation and automated evidence

The following verification was completed earlier in this session, before this
documentation-only update; these entries do not claim a new test run.

| Check or change | Recorded evidence |
| --- | --- |
| Shared frontend pacing | Immediate repaint requests; bounded recovery debt; fractional cycles retained; pause/focus reanchoring cancels old debt |
| Shared core hot path | Cached device deadlines; MMIO readback projection; region-based bus reads; power-of-two timer prescaler arithmetic |
| Native audio recovery | 40 ms startup threshold; transient underruns do not de-prime playback; event/callback diagnostics; 80 ms queue retained |
| Browser audio recovery | Matching 40 ms startup threshold and transient recovery; event/callback diagnostics; 80 ms queue retained |
| Workspace tests | PASS: 42 core unit tests, 1 backup integration test, 15 session tests, 2 app tests; 60 total |
| Recovery regressions | Meaningful RED before implementation, GREEN after implementation for host jitter/debt and native/browser audio recovery |
| Browser queue tests | PASS: 2 production-script tests; registered in CI |
| Native/WASM guest contracts | PASS: broad fixture runner; affine, affine-object, window, blend, raster, pulse, wave, noise, and cartridge timing/IRQ/keypad/DMA/PCM contracts |
| Static/build checks | PASS: strict workspace Clippy, formatting, native release app build, WASM app check, browser JavaScript syntax checks, and Trunk release build |
| FireRed bounded probe | 1,800 frames with fixed Start press at frames 360–361; BIOS enters cartridge; Flash128 detected; no execution error; reset retains backup bytes |
| FireRed before/after comparison | Three 1,800-frame runs per implementation; cycles 505612800, instructions 156596096, PC `0x08006bd0`, framebuffer generations 1800, PCM counters `(987525, 0, 14)` |
| Probe framebuffer SHA-256 | `e2cc2a1fa6131cf4d86faa3baf78851f35a36853e2467c257b3df9d89e85cce5` before and after |
| Sampled framebuffer equality | Exact bytes match before/after at frames 600, 1200, and 1800 in each comparison |
| PCM comparison | Matching non-cryptographic PCM fingerprint `eecebf9050561610` in each before/after run |
| Temporary automated logs | `/tmp/gba-firered-frame-timings.log`, `/tmp/gba-firered-probe-comparison.log`, `/tmp/gba-performance-contracts.log`; temporary local evidence, not tracked artifacts |

The bounded probe uses fresh cartridge storage and a fixed startup input. It
supports execution and performance verification, but does not independently
prove the plan's early-town/route, battle, save/reopen, or 30-minute live-session
acceptance scenarios.

### Manual acceptance and recorded limits

- [x] Slice 26 marked complete at the user's explicit request on 2026-10-02.
- [x] Linux native, Chrome, and Brave live FireRed snapshots recorded with exact timing values and audio counters.
- [x] ROM/BIOS identity, Flash128 selection, bounded probe, and earlier automated verification recorded.

Manual completion is attributed to the user's closure request. Exact town/route
and battle checkpoints, save-slot/reopen procedure, per-platform 30-minute
session logs, browser versions, display refresh, audio device, volume setting,
governor, thermals, and sustained wall-clock speed were not supplied and remain
not recorded. The native snapshot reports a restored backup; browser snapshots
report new cartridge storage. No additional browser save/reopen proof is
inferred from those labels. Audible stability of these specific latest runs
was not separately described; their nonzero underrun counters are retained.

This entry closes Slice 26 by user-declared acceptance and does not mark game
Slices 27–30 complete or establish equal performance across all five games.

## Slice 27 — Super Mario Advance 2: complete by user-confirmed manual acceptance

Record date: 2026-10-03. The user supplied the native, Chrome, and Brave
snapshots below, confirmed that the third capture was Brave, and explicitly
confirmed that all manual checks were completed and everything worked as
intended. Slice 27 is complete on that user-confirmed acceptance. The recorded
performance and cumulative audio counters remain separate from that acceptance.

### Cartridge, BIOS, and backup identity

ROM identity comes from the earlier exact-ROM headless reproduction in this
conversation; it was not rehashed during this documentation-only update. All
three supplied live snapshots report the same BIOS identity and backup selection.

| Field | Recorded value |
| --- | --- |
| Cartridge | `local-roms/super-mario-advance-2.gba` |
| ROM size | 4,194,304 bytes |
| ROM SHA-256 | `63d9fff04c635990a5c205a99ea64bfa698aa5cb9ec1333360063bbee949a4f3` |
| Header title / game code / revision | `SUPER MARIOB` / `AA2E` / 0 |
| BIOS | `private-data/gba_bios.bin`; user-supplied firmware through hardware reset-vector boot |
| BIOS size | 16,384 bytes |
| BIOS SHA-256 | `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570` |
| Cartridge backup | `Some(Eeprom)`; detected `Identified(Eeprom)`; override `None` |
| Resolved EEPROM capacity | 8,192 bytes on all three platforms |
| Native save directory used for the reproduced issue | `/tmp/lycan-sma2-clean` |
| Existing save envelope | `GBAEEPR1`; 8,232 bytes: 40-byte identity envelope plus 8,192-byte EEPROM payload |

### User-supplied live performance

Every value is copied exactly from the supplied diagnostics. Each metric has
120 samples. These snapshots are not a controlled replay of identical scenes,
inputs, save states, or session durations.

| Platform | Core mean/p95 ms | Pixel conversion mean/p95 ms | Texture submission mean/p95 ms | Samples (core / conversion / texture) |
| --- | --- | --- | --- | --- |
| Linux native UI | 9.476 / 16.500 | 0.082 / 0.200 | 0.004 / 0.000 | 120 / 120 / 120 |
| Chrome | 8.837 / 16.600 | 0.087 / 0.200 | 0.005 / 0.000 | 120 / 120 / 120 |
| Brave | 7.935 / 12.700 | 0.073 / 0.200 | 0.003 / 0.000 | 120 / 120 / 120 |

Texture submission mean/p95 pairs are retained exactly as supplied, including
p95 values below their means. All three core p95 values exceed the below-12 ms
goal and fall below the nominal 16.7427 ms GBA frame period. These individual
stage summaries do not establish complete application-frame time, GPU
completion, or sustained cycles per wall-clock second.

### Runtime snapshots supplied by the user

| Diagnostic | Linux native UI | Chrome | Brave |
| --- | ---: | ---: | ---: |
| Loaded ROM / execution state | `super-mario-advance-2.gba` / Running | `super-mario-advance-2.gba` / Running | `super-mario-advance-2.gba` / Running |
| Backup status | Saved revision 196 | Backup restored | Backup restored |
| Executed instructions | 241537606 | 622469400 | 672949460 |
| GBA cycles | 828010979 | 2093132180 | 2255449104 |
| Host output rate | 48000 Hz | 48000 Hz | 48000 Hz |
| Reported audio state | running | running | running |
| Current queue depth | 24.0 ms | 26.6 ms | 64.7 ms |
| Maximum queue depth | 52.5 ms | 47.1 ms | 77.3 ms |
| Queue capacity | 80 ms | 80 ms | 80 ms |
| Underrun events | 2 | 51 | 6248 |
| Underrun frames | 69 | 4183 | 797277 |
| Overflow events | 1 | 5 | 5 |
| Overflow frames | 394 | 4115 | 4115 |
| Audio callbacks | 15568 | 44068 | 54004 |
| Maximum callback size | 128 frames | 128 frames | 128 frames |
| Host output frames | 1949627 | 5634985 | 6111651 |
| Core PCM rate | 32768 Hz | 32768 Hz | 32768 Hz |
| Core PCM frames produced | 1617208 | 4088148 | 4405174 |
| PCM staging drops | 0 | 0 | 0 |
| Empty FIFO count | 12 | 41094 | 41094 |
| Backup / detection / override | `Some(Eeprom)` / `Identified(Eeprom)` / `None` | `Some(Eeprom)` / `Identified(Eeprom)` / `None` | `Some(Eeprom)` / `Identified(Eeprom)` / `None` |
| EEPROM capacity | 8192 bytes | 8192 bytes | 8192 bytes |
| Live pacing indicator | Not included in supplied snapshot | Not included in supplied snapshot | Not included in supplied snapshot |
| Native device errors | Not included in supplied snapshot | Not applicable | Not applicable |

At 48 kHz, the cumulative underrun frames correspond to approximately 0.001438
seconds of missing output on native, 0.087146 seconds on Chrome, and 16.609938
seconds on Brave. Event counts identify callbacks with missing output rather
than distinct audible interruptions. The snapshots do not identify when these
underruns, overflows, or empty FIFO reads occurred, or establish a cause. Brave's
large cumulative underrun count is retained despite the user's confirmation
that the manual checks worked as intended. Core PCM staging drops are zero on
all three platforms; host overflow counters are nonzero on all three.

### EEPROM fix and earlier automated verification

These results were obtained during the preceding fix in this conversation;
no code, builds, tests, emulator execution, or UI checks were rerun for this
documentation-only update.

| Check or change | Recorded evidence |
| --- | --- |
| Original failure reproduced | Exact ROM and BIOS; fresh EEPROM; 900 frames with Start pressed at frames 360–361; framebuffer displayed “Your saved data is corrupt.” |
| Protocol cause | 142 read commands rejected because their trailing clock carried bit 1; no trace overflow. Rejected commands returned ready bits rather than the requested block. |
| General EEPROM fix | Consume the final read-command clock regardless of its bit value; preserve write stop-bit validation, command/count checks, capacity selection, and save format. No title-specific override. |
| Reference comparison | mGBA `GBASavedataWriteEEPROM` transitions to a read response at the final command clock without validating that clock's value: https://github.com/mgba-emu/mgba/blob/master/src/gba/savedata.c |
| Regression justification | Existing serial DMA round-trip coverage only used a zero trailing bit. The new test catches the observed high trailing read clock returning ready bits instead of saved data. |
| RED / GREEN | `read_command_accepts_high_trailing_clock_without_changing_save` failed before the fix on the first dummy clock (`1` instead of `0`); passed after the fix for both 512-byte and 8-KiB devices, preserving bytes, revision, and clean state. |
| Workspace tests | PASS: 43 core unit tests, 1 backup integration test, 15 session tests, 2 app tests; 61 total |
| Trace-enabled core tests | PASS: 43 core unit tests and 1 backup integration test |
| Static/build checks | PASS: formatting, diff whitespace checks, strict Clippy for core/tools with trace enabled, core WASM check, and native release app build |
| Bounded trace | Compile-time `eeprom-trace` feature; opt-in core capture; 512 retained records between drains with explicit dropped-record counts; DMA3 directions, commands, responses, ready polling, and rejected commands recorded. Default production builds omit instrumentation. |
| Fixed fresh-save boot | 900 frames; world map reached; 392 responses and 196 write commands; zero response/image mismatches, rejected commands, or dropped trace records |
| Fixed separate restored-save boot | 900 frames; generated raw EEPROM restored; world map reached; 188 responses and zero write commands; zero response/image mismatches, rejected commands, or dropped trace records |
| Fixed existing-save boot | Copy of the user's original EEPROM payload restored; 900 frames; world map reached; 328 responses and 134 write commands; zero response/image mismatches, rejected commands, or dropped trace records. Original save file was left untouched. |
| Reset preservation | All three fixed probes returned PC to `0x00000000` and retained backup bytes |
| Fixed fresh-save framebuffer SHA-256 | `b1d08adf68a90546b4b10e586bc9fc134a96d44f9655f673dd24a2e38db10d91` |
| Fixed restored-save framebuffer SHA-256 | `c0bec781876ee6dfd1b79ef573841523f1e631521c3dbf1e184c76c789f01ed6` |
| Fixed existing-save framebuffer SHA-256 | `615881dcbdfe9602fe9071e9483553d0554d6dd2fdb3bdd7650be9e4fc54bec8` |
| Temporary evidence | `/tmp/sma2-first.log`; `/tmp/sma2-fixed-first.log`, `/tmp/sma2-fixed-second.log`, `/tmp/sma2-fixed-existing.log`; matching `.ppm`/`.png` captures and raw save payloads. Temporary local artifacts from the preceding fix, not tracked or rechecked for this update. |

Response/image comparisons used the initial raw image updated by each traced
write. They check transaction data, not independent hardware timing accuracy.
The headless probes establish the reproduced corruption fix and boot/restore
behavior; the gameplay and live-platform acceptance below comes from the user.

### Manual acceptance and recorded limits

- [x] Slice 27 complete by the user's explicit confirmation on 2026-10-03.
- [x] Agreed early-level gameplay and scrolling/action transitions manually verified.
- [x] Sprite/background composition and sound effects manually verified.
- [x] Detected EEPROM backup protocol and in-game save/resume manually verified.
- [x] Shared manual checks completed; the user reports everything working as intended.
- [x] Native, Chrome, and Brave snapshots recorded with exact timings, sample counts, backup states, and audio counters.
- [x] Earlier corruption reproduction, EEPROM fix, meaningful RED/GREEN regression, and automated checks recorded separately from manual acceptance.

The manual checklist is attributed to the user's statement that all checks
were completed. Exact level/checkpoint names, save-slot sequence, individual
shared-check observations, and per-platform long-session logs were not supplied
and remain not recorded. Sustained wall-clock speed, unthrottled speed, complete
frame/GPU timing, and memory/queue trends were not measured in the supplied
snapshots. Browser versions, refresh rate, zoom, audio device, volume setting,
power/governor state, thermals, and other host load were not refreshed for these
captures; the older reference environment is not a current-session measurement.

“Backup restored” changing to “Saved revision” is normal after guest writes and
host persistence. The native snapshot records revision 196; the Chrome and
Brave snapshots record successful restoration. These labels supplement the
user's manual save/resume confirmation rather than replacing it.

This entry closes Slice 27. It does not mark Slices 28–30 complete or establish
that the numerical performance targets were all met.

## Slice 28 — Metroid Fusion: complete by user-confirmed manual acceptance

Record date: 2026-10-03. The user supplied the native, Chrome, and Brave
captures below and explicitly confirmed that all manual checks were completed
and everything was working. Slice 28 is complete on that user-confirmed
acceptance. Platform labels are retained as supplied; no emulator or browser
was operated to independently reproduce these captures for this update.

### Cartridge, BIOS, and backup

| Field | Recorded value |
| --- | --- |
| Loaded cartridge | `metroid-fusion.gba` |
| ROM size / SHA-256 / header identity | Not recorded in the supplied snapshots; not inspected for this documentation-only update |
| BIOS size | 16,384 bytes on all three platforms |
| BIOS SHA-256 | `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570` on all three platforms |
| Cartridge backup | `Some(Sram)`; detected `Identified(Sram)`; override `None` on all three platforms |
| Backup status | Saved revision 1047 on all three platforms |
| Save directory / browser storage details | Not supplied |

### User-supplied live performance

Values are copied exactly from the supplied diagnostics. Every metric has 120
samples. Identical backup revision numbers do not establish identical inputs,
scenes, save bytes, or measurement windows across the three captures.

| Platform | Core mean/p95 ms | Pixel conversion mean/p95 ms | Texture submission mean/p95 ms | Samples (core / conversion / texture) |
| --- | --- | --- | --- | --- |
| Linux native UI | 6.972 / 8.119 | 0.078 / 0.090 | 0.002 / 0.003 | 120 / 120 / 120 |
| Chrome | 6.459 / 7.719 | 0.066 / 0.079 | 0.001 / 0.002 | 120 / 120 / 120 |
| Brave | 6.892 / 8.199 | 0.073 / 0.086 | 0.002 / 0.003 | 120 / 120 / 120 |

All three reported core p95 values meet the below-12 ms goal and are below the
nominal 16.7427 ms GBA frame period. Individual stage summaries do not establish
end-to-end application-frame time, GPU completion, or sustained emulation speed.

### Runtime snapshots supplied by the user

| Diagnostic | Linux native UI | Chrome | Brave |
| --- | ---: | ---: | ---: |
| Loaded ROM / execution state | `metroid-fusion.gba` / Running | `metroid-fusion.gba` / Running | `metroid-fusion.gba` / Running |
| Backup status | Saved revision 1047 | Saved revision 1047 | Saved revision 1047 |
| Executed instructions | 1034922793 | 1044290528 | 1047225613 |
| GBA cycles | 10145837393 | 10281144894 | 10323487513 |
| Host output rate | 48000 Hz | 48000 Hz | 48000 Hz |
| Current queue depth | 65.9 ms | 41.4 ms | 63.3 ms |
| Maximum queue depth | 72.1 ms | 72.1 ms | 72.1 ms |
| Queue capacity | 80 ms | 80 ms | 80 ms |
| Underrun events | 0 | 0 | 0 |
| Underrun frames | 0 | 0 | 0 |
| Overflow events | 0 | 0 | 0 |
| Overflow frames | 0 | 0 | 0 |
| Audio callbacks | 55701 | 58700 | 59751 |
| Maximum callback size | 512 frames | 512 frames | 512 frames |
| Host output frames | 28516864 | 28902400 | 29020672 |
| Reported device errors | 0 | 0 | 0 |
| Core PCM rate | 32768 Hz | 32768 Hz | 32768 Hz |
| Core PCM frames produced | 19816088 | 20080361 | 20163061 |
| PCM staging drops | 0 | 0 | 0 |
| Empty FIFO count | 14 | 14 | 14 |
| Backup / detection / override | `Some(Sram)` / `Identified(Sram)` / `None` | `Some(Sram)` / `Identified(Sram)` / `None` | `Some(Sram)` / `Identified(Sram)` / `None` |
| Explicit audio running/AudioContext state | Not included in supplied snapshot | Not included in supplied snapshot | Not included in supplied snapshot |
| Live pacing indicator | Not included in supplied snapshot | Not included in supplied snapshot | Not included in supplied snapshot |

All supplied snapshots report zero host audio underruns, overflows, device
errors, and core PCM staging drops. Current queue depths fall within the
40–80 ms target, and recorded maximum depths remain below the 80 ms capacity.
The empty FIFO count is 14 on every capture; its timing and cause are not
identified by these cumulative snapshots. Chrome and Brave labels are the
user's labels, including their reported 512-frame callback maximum and device
error field; these fields were not changed to match earlier browser captures.

### Manual acceptance and recorded limits

- [x] Slice 28 complete by the user's explicit manual confirmation on 2026-10-03.
- [x] Early exploration/combat checkpoint manually verified.
- [x] Room transitions manually verified as working.
- [x] Save-point use and save/resume manually verified.
- [x] Effects, composition, timing, and audio mixing manually verified.
- [x] Shared manual checks complete; the user reports everything working.
- [x] Native, Chrome, and Brave timings, sample counts, backup state, and audio counters recorded exactly.

Acceptance is attributed to the user's confirmation. Exact checkpoint/room
names, save-slot/reopen sequence, and individual shared-check observations were
not supplied and remain not recorded. No failing room transition was reported
in this request, and no regression replay was supplied or generated for this
update. The saved-revision labels report host persistence; manual save/resume
acceptance comes from the user's confirmation rather than those labels alone.

Browser versions, display refresh, zoom, audio device, volume setting, power
state, governor, thermals, and other host load were not refreshed for these
captures. Sustained wall-clock speed, unthrottled speed, complete frame/GPU
timing, and 30-minute memory/queue trends were not measured in the supplied
diagnostics. The older reference configuration is not a measurement of these
sessions. No new build, test, headless probe, or UI verification was performed
for this documentation-only update; earlier Slice 27 automated checks are not
presented as Metroid Fusion-specific verification.

This entry closes Slice 28 by user-confirmed manual acceptance. Slices 29–30
remain outside this update.

## Slice 29 — Zelda: The Minish Cap: complete by user-confirmed manual acceptance

Record date: 2026-10-03. The user supplied the native, Chrome, and Brave
captures below and confirmed that all checks were performed manually and that
everything was correct and smooth. Slice 29 is complete on that user-confirmed
acceptance. Platform labels and diagnostics are retained exactly as supplied;
no emulator or browser was operated for this documentation-only update.

### Cartridge, BIOS, and backup

| Field | Recorded value |
| --- | --- |
| Loaded cartridge | `minish-cap.gba` |
| ROM size / SHA-256 / header identity | Not recorded in the supplied snapshots; not inspected for this update |
| BIOS size | 16,384 bytes on all three platforms |
| BIOS SHA-256 | `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570` on all three platforms |
| Cartridge backup | `Some(Eeprom)`; detected `Identified(Eeprom)`; override `None` on all three platforms |
| Resolved EEPROM capacity | 8,192 bytes on all three platforms |
| Backup status | Saved revision 5 on all three platforms |
| Save directory / browser storage details | Not supplied |

### User-supplied live performance

Each metric contains 120 samples. The snapshots do not establish a controlled
replay of identical scenes, input sequences, save bytes, or measurement windows.

| Platform | Core mean/p95 ms | Pixel conversion mean/p95 ms | Texture submission mean/p95 ms | Samples (core / conversion / texture) |
| --- | --- | --- | --- | --- |
| Linux native UI | 7.458 / 9.560 | 0.061 / 0.078 | 0.001 / 0.002 | 120 / 120 / 120 |
| Chrome | 7.582 / 9.638 | 0.060 / 0.078 | 0.001 / 0.002 | 120 / 120 / 120 |
| Brave | 7.408 / 9.399 | 0.055 / 0.073 | 0.001 / 0.002 | 120 / 120 / 120 |

All three core p95 values meet the below-12 ms goal and fall below the nominal
16.7427 ms GBA frame period. These individual stage summaries do not establish
complete application-frame time, GPU completion, or sustained emulation speed.

### Runtime snapshots supplied by the user

| Diagnostic | Linux native UI | Chrome | Brave |
| --- | ---: | ---: | ---: |
| Loaded ROM / execution state | `minish-cap.gba` / Running | `minish-cap.gba` / Running | `minish-cap.gba` / Running |
| Backup status | Saved revision 5 | Saved revision 5 | Saved revision 5 |
| Executed instructions | 1047996625 | 1079728043 | 1101021090 |
| GBA cycles | 6524465500 | 6680941096 | 6780387869 |
| Host output rate | 48000 Hz | 48000 Hz | 48000 Hz |
| Current queue depth | 50.0 ms | 45.0 ms | 51.5 ms |
| Maximum queue depth | 55.8 ms | 55.8 ms | 55.8 ms |
| Queue capacity | 80 ms | 80 ms | 80 ms |
| Underrun events | 0 | 0 | 0 |
| Underrun frames | 0 | 0 | 0 |
| Overflow events | 0 | 0 | 0 |
| Overflow frames | 0 | 0 | 0 |
| Audio callbacks | 36172 | 38613 | 39914 |
| Maximum callback size | 512 frames | 512 frames | 512 frames |
| Host output frames | 18519040 | 18964992 | 19247616 |
| Reported device errors | 0 | 0 | 0 |
| Core PCM rate | 32768 Hz | 32768 Hz | 32768 Hz |
| Core PCM frames produced | 12743096 | 13048713 | 13242945 |
| PCM staging drops | 0 | 0 | 0 |
| Empty FIFO count | 13 | 13 | 13 |
| Backup / detection / override | `Some(Eeprom)` / `Identified(Eeprom)` / `None` | `Some(Eeprom)` / `Identified(Eeprom)` / `None` | `Some(Eeprom)` / `Identified(Eeprom)` / `None` |
| EEPROM capacity | 8192 bytes | 8192 bytes | 8192 bytes |
| Explicit audio running/AudioContext state | Not included in supplied snapshot | Not included in supplied snapshot | Not included in supplied snapshot |
| Live pacing indicator | Not included in supplied snapshot | Not included in supplied snapshot | Not included in supplied snapshot |

All three captures report zero host audio underruns, overflows, device errors,
and core PCM staging drops. Current queue depths meet the 40–80 ms target,
and the recorded maximum depth of 55.8 ms remains below the 80 ms capacity.
The cumulative empty FIFO count is 13 on every capture; these snapshots do not
identify its timing or cause. Chrome and Brave retain the user's platform
labels and their supplied 512-frame callback maximum and device-error field.

### Manual acceptance and recorded limits

- [x] Slice 29 complete by the user's explicit manual confirmation on 2026-10-03.
- [x] Early outdoor/interior areas and transitions manually verified.
- [x] Dialogue/action transitions manually verified.
- [x] In-game save and resume manually verified.
- [x] Layer/effect behavior, timing, and sound manually verified.
- [x] Shared manual checks complete; the user reports everything correct and smooth.
- [x] Native, Chrome, and Brave timings, sample counts, EEPROM selection/capacity, backup status, and audio counters recorded exactly.

The checklist is attributed to the user's confirmation that all checks were
completed. Exact area/checkpoint names, transition sequences, save-slot/reopen
procedure, and individual shared-check observations were not supplied and
remain not recorded. Saved revision 5 reports host persistence; save/resume
acceptance comes from the user's manual confirmation rather than that label
alone.

Browser versions, display refresh, zoom, audio device, volume setting, power
state, governor, thermals, and other host load were not refreshed for these
captures. Sustained wall-clock speed, unthrottled speed, complete frame/GPU
timing, and 30-minute memory/queue trends were not measured in the supplied
diagnostics. The older reference configuration is not a measurement of these
sessions. No new build, test, headless probe, or UI verification was performed
for this documentation-only update; earlier game-specific automated checks
are not presented as Minish Cap-specific verification.

This entry closes Slice 29 by user-confirmed manual acceptance. Slice 30
remains outside this update.
