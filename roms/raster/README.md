# Timed HBlank DMA raster effect

Original MIT ARM diagnostic with controlled startup, no BIOS, and no backup.
The guest builds one solid mode-0 background, then uses repeated HBlank DMA to
write BLDALPHA and BLDY for the next visible scanline.

The renderer completes the current scanline before the HBlank request. The
register values transferred during HBlank therefore affect the following line.
Each descriptor transfers two halfwords, reloads its destination to BLDALPHA,
reloads its count, and continues through its source table.

## States

| State | Active channels | Expected image |
| ---: | --- | --- |
| 0 | DMA0 | Brightness increases from top to bottom |
| 1 | DMA3 | Brightness decreases from top to bottom |
| 2 | DMA0 and DMA3 | Matches state 1 because DMA0 runs first and DMA3 writes the final values |

State 2 also requires mailbox IF snapshot `0x0900`, proving both channels
completed. The visual result alone does not establish that both transfers ran.
DMA0 writes BLDALPHA sentinel `0x0404`; DMA3 writes `0x0c0c`.

## Controls

- Right: next state
- Left: previous state

## Build and verify

From `gba-rs`:

```bash
python3 roms/raster/build.py
python3 roms/raster/verify.py
cargo run --locked -p gba-tools --release -- bench-raster
```

The first intentional build prints the ROM byte count and SHA-256. Copy those
values into `manifest.toml`; the builder never edits the frozen expectations.
Native captures are written to `target/raster-captures/frame-{12,13,17,18,22,23}.ppm`.
The independent oracle compares every pixel in all six frames.

## Manual acceptance

Run the native app:

```bash
cargo run --locked -p gba-app --release -- --debug-ui
```

Load `roms/raster.gba`, then verify the increasing gradient in state 0, the
decreasing gradient in state 1, and that state 2 matches state 1 while the
diagnostic reports DMA IF snapshot `0x0900`.

For Chrome and Brave, run the production web app from a second terminal:

```bash
env -u NO_COLOR trunk --config web/Trunk.toml serve --release --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080`, click **Load ROM**, and select
`roms/raster.gba`. Repeat the three state checks in Chrome and Brave. Record
the browser and OS versions and whether the diagnostic label and all three
images matched.
