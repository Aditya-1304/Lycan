# Supplied BIOS startup and bounded game probes

Slice 20 implementation is ready for supplied-file verification. Actual retail
BIOS execution, the upstream BIOS guest result, and all five game observations
remain **not verified**. No private BIOS or commercial ROM path was supplied for
implementation checks. Build and synthetic core evidence cannot establish compatibility.

The native and browser apps share **Load BIOS**, size validation, SHA-256 status,
and **Load ROM**. Images outside the byte-identical shipped diagnostic set boot
from PC 0 in ARM supervisor mode with interrupts disabled and uninitialized
stacks. The BIOS establishes the cartridge runtime state. Controlled diagnostics
retain their original route. SWI and IRQ dispatch guest firmware instructions;
there are no host BIOS service replacements.

Reset retains BIOS and cartridge bytes, backup contents, revision and dirty
state. It clears CPU/RAM/device state, serial command state, Flash bank selection,
input, staged audio and host pacing. Replacing a BIOS reboots the active session
with its backup retained; an existing pause/save barrier stays paused.

BIOS protection uses execution context independently of pipeline lookahead.
Successful BIOS opcode fetches update the protected word latch. Outside-BIOS
reads select byte/halfword/word lanes from that latch. CPU open-bus reads use
instruction prefetch context; DMA uses its data bus latch. Existing cartridge
wait-state and prefetch behavior remains covered by earlier timing fixtures.

## BIOS diagnostic

The MIT upstream `jsmolka/gba-tests` diagnostic is pinned at
`a7113b67e63f83a9b321696ddd7042ccfad6c881`. `roms/gba-tests/bios/bios.gba`
is 1908 bytes, SHA-256
`9d7b369fa1aa661ff03692b3d79c6f644b623d72983d0fc890e6d87a0409a3c9`.
Its source checks startup, sqrt SWI, in-IRQ and post-IRQ protected reads. Success
requires PC `0x080003c0`, r12 = 0, and the independently frozen upstream
“All tests passed” framebuffer hash
`59ce42abae9825c2d2579c5cd838e47d88be917e37ea36ff162d46fc5d0991e3`.
The shared native/WASM contract caps execution at 50 million instructions and
600 frames (168537600 cycles), including subsequent complete scanout.
Firmware is read at runtime, never embedded in the WASM artifact.

Run this gate before investigating games. Replace only the BIOS path:

```bash
cd /home/aditya/Projects/GBA/gba-rs
BIOS_PATH="/absolute/path/to/gba_bios.bin"
cargo run --locked -p gba-tools --release -- verify-bios --bios "$BIOS_PATH" --capture /tmp/bios-pass.ppm
python3 roms/bios/verify_wasm.py --bios "$BIOS_PATH"
```

The following mutation runs must fail after a valid baseline passes. They change
the expected framebuffer, exercising result validation rather than accepting an
arbitrary terminal loop:

```bash
cargo run --locked -p gba-tools --release -- verify-bios --bios "$BIOS_PATH" --expected-frame-sha256 0000000000000000000000000000000000000000000000000000000000000000
python3 roms/bios/verify_wasm.py --bios "$BIOS_PATH" --expected-frame-sha256 0000000000000000000000000000000000000000000000000000000000000000
```

Size/missing-firmware rejection, reset/save preservation, and protected byte
lanes have focused core tests. Guest-executed block-load open-bus and paused BIOS
installation regressions were observed failing before their fixes and passing
afterward. These synthetic checks are distinct from the retail BIOS gate.

## Five-game evidence

Keep files in ignored `private-data/` or `local-roms/`. The probe prints original
ROM and BIOS hashes, title/game code/revision header fields, detector evidence,
PC, cycles, instructions, frame generation, framebuffer hash, PCM counters,
first execution error, and reset/save-preservation result. Record region from
your cartridge identity alongside the game code. An unknown/ambiguous detector
result must stay unresolved unless a documented `--backup` override is supplied.

| Game | ROM region/revision/size/hash | BIOS hash | Save detection | Startup/input/audio | Reset/save | Linux/Chrome/Brave |
| --- | --- | --- | --- | --- | --- | --- |
| Pokémon FireRed | not recorded | not recorded | not verified | not verified | not verified | not verified |
| Pokémon Emerald | not recorded | not recorded | not verified | not verified | not verified | not verified |
| Zelda: The Minish Cap | not recorded | not recorded | not verified | not verified | not verified | not verified |
| Metroid Fusion | not recorded | not recorded | not verified | not verified | not verified | not verified |
| Super Mario Advance 2 | not recorded | not recorded | not verified | not verified | not verified | not verified |

Use your own filenames here. Each probe defaults to 600 frames with a 200000
instruction cap per frame; `--frames` accepts 1–1800. `--press-start` holds Start
at the beginning of frame 360 and releases it at frame 362. This is an exploratory
input, not a frozen title-specific checkpoint. A successful command means only
that bounded execution entered ROM without a core error and retained backup on
reset. A frame generation/hash alone does not prove a visible game scene.

```bash
ROM_DIR="/absolute/path/to/your/roms"
cargo run --locked -p gba-tools --release -- probe --bios "$BIOS_PATH" --rom "$ROM_DIR/firered.gba" --press-start --capture /tmp/firered.ppm
cargo run --locked -p gba-tools --release -- probe --bios "$BIOS_PATH" --rom "$ROM_DIR/emerald.gba" --press-start --capture /tmp/emerald.ppm
cargo run --locked -p gba-tools --release -- probe --bios "$BIOS_PATH" --rom "$ROM_DIR/minish-cap.gba" --press-start --capture /tmp/minish-cap.ppm
cargo run --locked -p gba-tools --release -- probe --bios "$BIOS_PATH" --rom "$ROM_DIR/metroid-fusion.gba" --press-start --capture /tmp/metroid-fusion.ppm
cargo run --locked -p gba-tools --release -- probe --bios "$BIOS_PATH" --rom "$ROM_DIR/super-mario-advance-2.gba" --press-start --capture /tmp/sma2.ppm
```

After the BIOS gate passes, open the apps manually:

```bash
cargo run --locked -p gba-app --release
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080` in Chrome and Brave. Load BIOS first, then each game.
Check a visible startup scene, Start/menu input, audible sound, pause/resume,
focus/visibility return, reset through BIOS, and existing save data after reset.
Use Z/X, Enter/Backspace, arrows and A/S for A/B, Start/Select, directions and L/R.
Record screenshots and the first error/status if a game stops. Preserve any save
export before a repro. Later graphics/RTC dependencies need a bounded reproduction
before being pulled forward. Slice 21 has not been started.
