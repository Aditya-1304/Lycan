#!/usr/bin/env python3
"""Rebuild both original EEPROM score cartridges and check frozen identities."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
manifest = root / "roms/eeprom/manifest.json"
expected = json.loads(manifest.read_text())
with tempfile.TemporaryDirectory(prefix="gba-eeprom-") as temporary:
    for name, large in [("eeprom512-score", False), ("eeprom8k-score", True)]:
        obj, elf = Path(temporary) / "score.o", Path(temporary) / "score.elf"
        args = ["arm-none-eabi-as", "-mcpu=arm7tdmi"]
        if large:
            args += ["--defsym", "LARGE=1"]
        subprocess.run(args + ["-o", str(obj), str(root / "roms/eeprom/score.s")], check=True)
        subprocess.run(["arm-none-eabi-ld", "-T", str(root / "roms/pixels/linker.ld"), "-o", str(elf), str(obj)], check=True)
        rom = root / "roms" / (name + ".gba")
        subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(elf), str(rom)], check=True)
        identity = hashlib.sha256(rom.read_bytes()).hexdigest()
        assert identity == expected[name]["sha256"], "EEPROM ROM differs from frozen identity"
        print(name, len(rom.read_bytes()), identity)
