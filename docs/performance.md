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
