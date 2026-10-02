#!/usr/bin/env python3

"""Reproducibly assemble the original Slice-24 blend guest."""

from pathlib import Path
import hashlib
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
fixture = Path(__file__).resolve().parent
output = root / "roms/blend.gba"

with tempfile.TemporaryDirectory(prefix="gba-blend-") as temporary:
    temporary = Path(temporary)

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

    print(subprocess.check_output(
        ["arm-none-eabi-nm", "-n", str(elf)],
        text=True,
    ))

    print(f"bytes = {len(data)}")
    print(f'sha256 = "{hashlib.sha256(data).hexdigest()}"')