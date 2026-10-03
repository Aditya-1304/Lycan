#!/usr/bin/env python3
"""Assemble the original Flash128 score cartridge and verify its frozen identity."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[3]
with tempfile.TemporaryDirectory(prefix="gba-flash-") as tmp:
    obj, elf = Path(tmp) / "score.o", Path(tmp) / "score.elf"
    subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "-o", str(obj), str(root / "fixtures/diagnostics/flash-banked/score.s")], check=True)
    subprocess.run(["arm-none-eabi-ld", "-T", str(root / "fixtures/diagnostics/pixels/linker.ld"), "-o", str(elf), str(obj)], check=True)
    subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(elf), str(root / "fixtures/diagnostics/flash-banked-score.gba")], check=True)
identity = hashlib.sha256((root / "fixtures/diagnostics/flash-banked-score.gba").read_bytes()).hexdigest()
assert identity == json.loads((root / "fixtures/diagnostics/flash-banked/manifest.json").read_text())["sha256"], "Flash128 ROM differs from frozen identity"
print("Flash128 score cartridge rebuilt; frozen SHA-256 matches", identity)
