# Wave effect fixture

Original MIT guest scene, using skip-BIOS startup with explicit test firmware.
Both pulse voices accompany channel 3 on the left; wave output is on the right.
Z selects a second waveform and adds the existing DMA/FIFO PCM to both sides.
X selects bank 1, Backspace selects 64-digit playback, Up forces 75% volume,
and Down disables the master mixer. Release retriggers the scene. The colored
square follows the accompaniment note every 32 VBlanks.

The guest fills each opposite bank through wave RAM, then triggers playback
with byte writes that must retain the write-only frequency latch. The bounded
48-frame contract checks register masks/status, both bank readbacks, recorded
8192/4096 Hz periods, gain, pulse/PCM mixing, mute/restart, full scanout and
HALT completion. Identical output across frame and 997-cycle execution chunks,
and resampling at 44.1/48/96 kHz, establishes continuous streaming history.
The manifest freezes the ROM and accepted PCM16 WAV hashes.

```bash
python3 roms/wave/build.py
python3 roms/wave/verify.py
cargo run --locked -p gba-tools --release -- verify-wave --capture /tmp/wave.wav
```

Native and browser listening, lifecycle checks and app timings remain manual.
