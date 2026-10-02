#!/usr/bin/env python3
"""Reproducibly assemble the original window guest without changing expectations."""
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[2]
with tempfile.TemporaryDirectory(prefix="gba-window-") as temporary:
    obj = Path(temporary) / "scene.o"
    elf = Path(temporary) / "scene.elf"
    subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "-o", str(obj), str(root / "roms/window/scene.s")], check=True)
    subprocess.run(["arm-none-eabi-ld", "-T", str(root / "roms/pixels/linker.ld"), "-o", str(elf), str(obj)], check=True)
    subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(elf), str(root / "roms/window.gba")], check=True)
    print(subprocess.check_output(["arm-none-eabi-nm", "-n", str(elf)], text=True))
