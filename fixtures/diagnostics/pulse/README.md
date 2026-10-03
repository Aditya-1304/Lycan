# Pulse melody fixture

Original MIT guest fixture; no supplied BIOS is required. The app enables its
explicit test firmware, and the contract enables the same firmware in Machine.

The scene plays four lead notes (256, about 322.8, about 384.4, and 512 Hz),
with a lower second voice. Notes change every 32 VBlanks. A 16-by-16 square
changes red, green, blue and white with the notes. Lead is routed right and the
second voice left. Z adds the existing 2048 Hz DMA/FIFO PCM tone to both sides.

Controls: X selects a decreasing channel-1 sweep; Backspace selects 25% duty;
Up selects a decreasing envelope; Down disables the master mixer. Releasing a
control retriggers the voices. Enable host audio in the app before listening.

```bash
python3 fixtures/diagnostics/pulse/build.py
python3 fixtures/diagnostics/pulse/verify.py
cargo run --locked -p gba-tools --release -- verify-pulse --capture /tmp/pulse.wav
```

`manifest.toml` freezes the ROM and accepted PCM16 WAV identities, startup,
completion range and bounds. `contract.rs` checks exact note registers/mailbox,
full scanout, measured recorded periods, duty/envelope/sweep controls, silence
and restart, zero dropped samples/underruns, and identical streams for frame
and 997-cycle chunks. The production resampler is checked at 44.1/48/96 kHz.
`verify.py` runs the same guest contract natively and in the production WASM
core. These checks prove the emulated signal; audible native/browser acceptance
and app millisecond timings remain manual.
