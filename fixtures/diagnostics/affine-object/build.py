#!/usr/bin/env python3
"""Assemble the original guest and pack its source into four OBJ layouts."""
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[3]
with tempfile.TemporaryDirectory(prefix="gba-object-") as temporary:
    directory = Path(temporary)
    layouts = bytearray()
    for mapping_2d in (False, True):
        for depth in (8, 4):
            data = bytearray(4096)
            for y in range(32):
                for x in range(32):
                    # Transparent asymmetric notch plus distinct source-coordinate colors.
                    color = 0 if x < 6 or (x < 16 and y < 12) else 1 + (x // 4 + 3 * (y // 4)) % 15
                    stride = 32 if mapping_2d else 4 * (depth // 4)
                    tile = y // 8 * stride + x // 8 * (depth // 4)
                    offset = tile * 32 + (y % 8) * depth + (x % 8) * depth // 8
                    data[offset] |= color << ((x % 2) * 4) if depth == 4 else color
            layouts.extend(data)
    (directory / "texture.bin").write_bytes(layouts)
    subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "-o", str(directory / "sprite.o"), str(root / "fixtures/diagnostics/affine-object/sprite.s")], cwd=directory, check=True)
    subprocess.run(["arm-none-eabi-ld", "-T", str(root / "fixtures/diagnostics/pixels/linker.ld"), "-o", str(directory / "sprite.elf"), str(directory / "sprite.o")], check=True)
    subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(directory / "sprite.elf"), str(root / "fixtures/diagnostics/affine-object.gba")], check=True)
    print(subprocess.check_output(["arm-none-eabi-nm", "-n", str(directory / "sprite.elf")], text=True))
    subprocess.run(["arm-none-eabi-as", "-mcpu=arm7tdmi", "-o", str(directory / "degenerate.o"), str(root / "fixtures/diagnostics/affine-object/degenerate.s")], check=True)
    subprocess.run(["arm-none-eabi-ld", "-T", str(root / "fixtures/diagnostics/pixels/linker.ld"), "-o", str(directory / "degenerate.elf"), str(directory / "degenerate.o")], check=True)
    subprocess.run(["arm-none-eabi-objcopy", "-O", "binary", str(directory / "degenerate.elf"), str(root / "fixtures/diagnostics/affine-object-degenerate.gba")], check=True)
    print(subprocess.check_output(["arm-none-eabi-nm", "-n", str(directory / "degenerate.elf")], text=True))
