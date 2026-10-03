#!/usr/bin/env python3
"""Reproducibly assemble the original window guest without changing expectations."""
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[3]
with tempfile.TemporaryDirectory(prefix="gba-window-") as temporary:
    obj = Path(temporary) / "scene.o"
    elf = Path(temporary) / "scene.elf"
    subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "-o", str(obj), str(root / "fixtures/diagnostics/window/scene.s")], check=True)
    subprocess.run(["arm-none-eabi-ld", "-T", str(root / "fixtures/diagnostics/pixels/linker.ld"), "-o", str(elf), str(obj)], check=True)
    subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(elf), str(root / "fixtures/diagnostics/window.gba")], check=True)
    print(subprocess.check_output(["arm-none-eabi-nm", "-n", str(elf)], text=True))
