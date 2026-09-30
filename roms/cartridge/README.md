# Cartridge timing diagnostic

This original MIT fixture executes 24 bounded cases: ARM (32-bit fetches) and
Thumb (16-bit fetches), in each WS0/WS1/WS2 ROM alias, with default or programmed
waitstates and prefetch disabled or enabled. Every case performs four EWRAM
word loads interleaved with register moves, then a cartridge halfword load and
word load. EWRAM accesses and load-internal cycles let the opcode FIFO fill;
cartridge data reads discard it. Setup uses BX to redirect the fetch stream.

The manifest pins the ROM SHA-256, marker PCs, instruction/cycle limits,
completion PC, and independently calculated elapsed cycles. The interval starts
**after** executing the first marker and ends **after** executing the second;
this includes their pipeline fetch effects. The guest writes expected counts to
IWRAM at `0x03000008 + 2 * case_index`. The host measures actual emulated cycles
and compares them with the manifest. No emulated timer is required. Completion
requires mailbox ID `0x0060`, result `1`, and loaded ROM data `0x12345678`.

Timing definitions follow [GBATEK](https://mgba-emu.github.io/gbatek/) and the
[prefetch explanation from mGBA](https://mgba.io/2015/06/27/cycle-counting-prefetch/).
N and S below include the transfer cycle. Defaults are N=5 and S=3/5/9 for
WS0/WS1/WS2. WAITCNT `0x06da` selects N=3 and S=2 in every window. Adding
`0x4000` enables the eight-halfword FIFO. SRAM waitstate selectors are checked
by the focused core regression test; this ROM does not access save memory.

## Independent cycle accounting

Without prefetch, each ARM EWRAM load/move pair costs `N + 3S + 7`.
The final ROM halfword load, word load, and marker cost respectively
`N + 2S + 1`, `2N + 2S + 1`, and `N + S`.
The ARM total is therefore `8N + 17S + 30`.
For Thumb the pair costs `N + S + 7`, and the final operations cost
`N + S + 1`, `2N + S + 1`, and `N`, totaling `8N + 6S + 30`.

With prefetch, seven cartridge-free cycles follow each EWRAM word load.
A buffered instruction takes one CPU cycle; an incomplete halfword waits for
its remaining S cycles. ARM consumes two halfwords per fetch, Thumb one.
The following decomposition supplies the enabled expectations without recording
emulator output as an oracle. Each cell lists EWRAM pairs + ROM halfword load +
ROM word load + final marker.

| Setting | Window | ARM | Thumb |
| --- | --- | --- | --- |
| Default + prefetch | WS0 | 50 + 10 + 17 + 8 = 85 | 38 + 7 + 14 + 5 = 64 |
| Default + prefetch | WS1 | 80 + 16 + 21 + 10 = 127 | 43 + 8 + 16 + 5 = 72 |
| Default + prefetch | WS2 | 144 + 24 + 29 + 14 = 211 | 72 + 15 + 20 + 5 = 112 |
| Programmed + prefetch | All | 39 + 5 + 11 + 5 = 60 | 37 + 5 + 9 + 3 = 54 |

These are reference-derived expectations, not measurements from physical GBA
hardware. The implemented scope covers these access sequences, FIFO saturation,
partial fills, cartridge-data cancellation, WAITCNT writes, redirects, and
128 KiB nonsequential boundaries. It does not establish every undocumented
prefetch interaction or video-bus contention rule.

## Repeat validation

Run from the repository root:

```sh
cargo run --locked -p gba-tools -- build-fixtures
cargo run --locked -p gba-tools --release -- fixtures run
python3 roms/cartridge/verify_wasm.py
python3 roms/cartridge/verify_wasm.py --diagnostics
```

The WASM verifier requires Python 3.11+, Node 26.10.0, the pinned Rust toolchain,
and the `wasm32-unknown-unknown` target.
It executes the same core library used by the browser app and uses the manifest
as its expected-result source. It checks all intervals, total execution bounds,
the terminal PC, and every mailbox result. It does not exercise browser UI.
`verify_wasm.py` builds the production `gba-core` rlib for WASM, links the small
`wasm_harness.rs` bridge, and runs `verify_wasm.mjs` in Node. The bridge owns one
machine across all 24 intervals; every marker call uses the remaining total
instruction budget and absolute cycle limit. Rust failures trap WASM and Node
assertion failures return a nonzero status. Expectations are parsed directly
from `fixtures/manifest.toml`; none are duplicated in the bridge.

The optional `--diagnostics` check additionally executes the pinned ARM, Thumb,
services and memory guests in WASM. It verifies terminal opcode, firmware/ROM
hashes, CPU status and result registers, shared execution limits, and the frozen
success-screen digest after complete scanout where the manifest declares one.
CI runs this extended command. It strengthens cross-target guest verification;
Node execution does not establish Chrome/Brave rendering or interaction acceptance.

Earlier calculations/copy, the ARM/Thumb counter, complete upstream ARM/Thumb
diagnostics, tiled/stripes scenes, sprites and memory diagnostics remain in the
normal fixture run. See [the closeout record](../../docs/verification/slices_0_9.md)
for runtime results and the remaining user-performed platform checks.
