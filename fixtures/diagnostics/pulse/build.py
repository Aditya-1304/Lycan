#!/usr/bin/env python3
"""Rebuild the original pulse cartridge and require its frozen SHA-256 identity."""
import hashlib
import tomllib
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[3]
expected = tomllib.loads((root / "fixtures/diagnostics/pulse/manifest.toml").read_text())
with tempfile.TemporaryDirectory(prefix="gba-pulse-") as temporary:
    obj, elf = Path(temporary) / "pulse.o", Path(temporary) / "pulse.elf"
    subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "-o", str(obj), str(root / "fixtures/diagnostics/pulse/melody.s")], check=True)
    subprocess.run(["arm-none-eabi-ld", "-T", str(root / "fixtures/diagnostics/pixels/linker.ld"), "-o", str(elf), str(obj)], check=True)
    rom = root / "fixtures/diagnostics/pulse.gba"
    subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(elf), str(rom)], check=True)
    identity = hashlib.sha256(rom.read_bytes()).hexdigest()
    assert identity == expected["sha256"], "Pulse cartridge differs from frozen identity"
    print("PASS pulse ROM", len(rom.read_bytes()), identity)
