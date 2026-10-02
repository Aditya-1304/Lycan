#!/usr/bin/env python3

"""Reproducibly assemble the original Slice-25 raster/HBlank-DMA guest."""

from pathlib import Path
import hashlib
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
fixture = Path(__file__).resolve().parent
output = root / "roms/raster.gba"


def make_table(sentinel: int, reverse: bool) -> bytes:
    data = bytearray()

    # Entry N is consumed during HBlank of visible line N and therefore
    # supplies the effect state for visible line N+1.
    for hblank_line in range(160):
        next_visible_line = min(hblank_line + 1, 159)
        level = next_visible_line // 10

        if reverse:
            level = 15 - level

        data.extend(sentinel.to_bytes(2, "little"))
        data.extend(level.to_bytes(2, "little"))

    assert len(data) == 160 * 4
    return bytes(data)


with tempfile.TemporaryDirectory(prefix="gba-raster-") as temporary:
    temporary = Path(temporary)

    (temporary / "dma0-table.bin").write_bytes(
        make_table(0x0404, reverse=False)
    )

    (temporary / "dma3-table.bin").write_bytes(
        make_table(0x0C0C, reverse=True)
    )

    obj = temporary / "scene.o"
    elf = temporary / "scene.elf"

    subprocess.run(
        [
            "arm-none-eabi-as",
            "-mcpu=arm7tdmi",
            "-o",
            str(obj),
            str(fixture / "scene.s"),
        ],
        cwd=temporary,
        check=True,
    )

    subprocess.run(
        [
            "arm-none-eabi-ld",
            "-T",
            str(root / "roms/pixels/linker.ld"),
            "-o",
            str(elf),
            str(obj),
        ],
        check=True,
    )

    subprocess.run(
        [
            "arm-none-eabi-objcopy",
            "-O",
            "binary",
            str(elf),
            str(output),
        ],
        check=True,
    )

    data = output.read_bytes()

    print(
        subprocess.check_output(
            ["arm-none-eabi-nm", "-n", str(elf)],
            text=True,
        )
    )

    print(f"bytes = {len(data)}")
    print(f'sha256 = "{hashlib.sha256(data).hexdigest()}"')
