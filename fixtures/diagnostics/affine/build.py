#!/usr/bin/env python3
"""Reproducibly assemble the five original affine image variants."""
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[3]
with tempfile.TemporaryDirectory(prefix="gba-affine-") as temporary:
    obj, elf = Path(temporary) / "image.o", Path(temporary) / "image.elf"
    for mode in range(1, 6):
        subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "--defsym", f"MODE={mode}", "-o", str(obj), str(root / "fixtures/diagnostics/affine/image.s")], check=True)
        subprocess.run(["arm-none-eabi-ld", "-T", str(root / "fixtures/diagnostics/pixels/linker.ld"), "-o", str(elf), str(obj)], check=True)
        subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(elf), str(root / f"fixtures/diagnostics/affine-mode-{mode}.gba")], check=True)
