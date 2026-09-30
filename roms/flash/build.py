#!/usr/bin/env python3
"""Assemble the original Flash64 score cartridge and verify its frozen identity."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[2]
with tempfile.TemporaryDirectory(prefix="gba-flash-") as tmp:
    obj, elf = Path(tmp) / "score.o", Path(tmp) / "score.elf"
    subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "-o", str(obj), str(root / "roms/flash/score.s")], check=True)
    subprocess.run(["arm-none-eabi-ld", "-T", str(root / "roms/pixels/linker.ld"), "-o", str(elf), str(obj)], check=True)
    subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(elf), str(root / "roms/flash-score.gba")], check=True)
identity = hashlib.sha256((root / "roms/flash-score.gba").read_bytes()).hexdigest()
assert identity == json.loads((root / "roms/flash/manifest.json").read_text())["sha256"], "Flash64 ROM differs from frozen identity"
print("Flash64 score cartridge rebuilt; frozen SHA-256 matches", identity)
