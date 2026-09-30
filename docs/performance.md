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
| Native Linux | Not measured | Not measured | Not measured | Not recorded / Not recorded / Not recorded | Pending user check |
| Google Chrome | Not measured | Not measured | Not measured | Not recorded / Not recorded / Not recorded | Pending user check |
| Brave | Not measured | Not measured | Not measured | Not recorded / Not recorded / Not recorded | Pending user check |

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
