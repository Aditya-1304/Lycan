#!/usr/bin/env python3
"""Rebuild the original pulse cartridge and require its frozen SHA-256 identity."""
import hashlib
import tomllib
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
expected = tomllib.loads((root / "roms/pulse/manifest.toml").read_text())
with tempfile.TemporaryDirectory(prefix="gba-pulse-") as temporary:
    obj, elf = Path(temporary) / "pulse.o", Path(temporary) / "pulse.elf"
    subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "-o", str(obj), str(root / "roms/pulse/melody.s")], check=True)
    subprocess.run(["arm-none-eabi-ld", "-T", str(root / "roms/pixels/linker.ld"), "-o", str(elf), str(obj)], check=True)
    rom = root / "roms/pulse.gba"
    subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(elf), str(rom)], check=True)
    identity = hashlib.sha256(rom.read_bytes()).hexdigest()
    assert identity == expected["sha256"], "Pulse cartridge differs from frozen identity"
    print("PASS pulse ROM", len(rom.read_bytes()), identity)
