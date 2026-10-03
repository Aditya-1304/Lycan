# Noise scene fixture

Original MIT cartridge using explicit test firmware, not a supplied BIOS.
A red/green square changes every 32 VBlanks. Each event triggers a timed noise
effect and swaps FIFO A/B between timer 0 and cascaded timer 1. DMA1 and DMA2
refill independently; timer 2 cascades from timer 1 at /3 and timer 3 from timer
2 at /4. Timer IF flags remain latched for the acceptance contract.

Both pulse voices, wave and both PCM FIFOs continue under each noise burst.
Z changes the paired event immediately, X selects seven-bit noise, and
Backspace isolates noise while all devices retain their clocks. Enable audio
in the app before listening. Pause, focus loss, mute and reset use existing
frontend lifecycle controls.

```bash
python3 fixtures/diagnostics/noise/build.py
python3 fixtures/diagnostics/noise/verify.py
cargo run --locked -p gba-tools --release -- verify-noise --capture /tmp/noise.wav
cargo run --locked -p gba-app --release -- --audio-probe noise
```

The bounded native/WASM contract executes 1800 frames at frame-sized and
997-cycle deadlines, checks full scanout, mailbox, terminal HALT, all cascade
counters, DMA descriptors, independently measured PCM clock changes, noise
length expiry and the 127-edge short-mode signal period. All four PSG activity
flags must be present during the effect. Exact stereo streams and resampler
output counts agree across chunks at 44.1/48/96 kHz. The frozen WAV is the
30.137-second guest recording; it is not a host-device recording.

The focused core regression checks noise envelope decay, length expiry,
DAC/master disable, register readback and oscillator history. The existing
FIFO regression now also checks DMA2, both destinations and all PWM modes.
The device probe measures real native queues for 30 seconds plus two seconds
of resumed playback, including pause/resume, reset and focus suspension.
These bounded checks do not replace manual listening or a 30-minute app run.
