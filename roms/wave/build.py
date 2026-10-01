#!/usr/bin/env python3
"""Rebuild the original wave cartridge and require its frozen SHA-256 identity."""
import hashlib
import tomllib
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
expected = tomllib.loads((root / "roms/wave/manifest.toml").read_text())
with tempfile.TemporaryDirectory(prefix="gba-wave-") as temporary:
    obj, elf = Path(temporary) / "wave.o", Path(temporary) / "wave.elf"
    subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "-o", str(obj), str(root / "roms/wave/effect.s")], check=True)
    subprocess.run(["arm-none-eabi-ld", "-T", str(root / "roms/pixels/linker.ld"), "-o", str(elf), str(obj)], check=True)
    rom = root / "roms/wave.gba"
    subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(elf), str(rom)], check=True)
    identity = hashlib.sha256(rom.read_bytes()).hexdigest()
    assert identity == expected["sha256"], "Wave cartridge differs from frozen identity"
    print("PASS wave ROM", len(rom.read_bytes()), identity)
