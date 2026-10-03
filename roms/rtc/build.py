#!/usr/bin/env python3
"""Rebuild the original RTC diagnostic with the shared cartridge linker."""
from pathlib import Path
import subprocess
import tempfile
import hashlib
root = Path(__file__).resolve().parents[2]
with tempfile.TemporaryDirectory(prefix="gba-rtc-") as tmp:
    obj, elf = Path(tmp) / "clock.o", Path(tmp) / "clock.elf"
    subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "-o", str(obj), str(root / "roms/rtc/clock.s")], check=True)
    subprocess.run(["arm-none-eabi-ld", "-T", str(root / "roms/pixels/linker.ld"), "-o", str(elf), str(obj)], check=True)
    subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(elf), str(root / "roms/rtc.gba")], check=True)
    print(subprocess.check_output(["arm-none-eabi-nm", "-n", str(elf)], text=True))
print(hashlib.sha256((root / "roms/rtc.gba").read_bytes()).hexdigest())
