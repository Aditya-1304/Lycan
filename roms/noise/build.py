#!/usr/bin/env python3
"""Rebuild the original noise cartridge and require its frozen SHA-256 identity."""
import hashlib
import tomllib
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
expected = tomllib.loads((root / "roms/noise/manifest.toml").read_text())
with tempfile.TemporaryDirectory(prefix="gba-noise-") as temporary:
    obj, elf = Path(temporary) / "noise.o", Path(temporary) / "noise.elf"
    subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "-o", str(obj), str(root / "roms/noise/effect.s")], check=True)
    subprocess.run(["arm-none-eabi-ld", "-T", str(root / "roms/pixels/linker.ld"), "-o", str(elf), str(obj)], check=True)
    rom = root / "roms/noise.gba"
    subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(elf), str(rom)], check=True)
    identity = hashlib.sha256(rom.read_bytes()).hexdigest()
    assert identity == expected["sha256"], "Noise cartridge differs from frozen identity"
    print("PASS noise ROM", len(rom.read_bytes()), identity)
