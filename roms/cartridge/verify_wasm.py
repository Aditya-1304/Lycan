#!/usr/bin/env python3
"""Build the production core for WASM and execute its frozen timing contract.

Requires Python 3.11+, Node 26.10.0, and the pinned Rust toolchain with the WASM
target. Cargo's artifact record locates the actual rlib, including when callers
override CARGO_TARGET_DIR. Temporary bridge artifacts never modify ROMs.
"""

import hashlib
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--diagnostics", action="store_true",
                        help="also execute ARM, Thumb, services and memory contracts in WASM")
    parser.add_argument("--vblank", action="store_true",
                        help="also execute the VBlank IRQ/HALT guest contract in WASM")
    parser.add_argument("--keypad", action="store_true",
                        help="also execute both keypad IRQ/HALT and framebuffer contracts in WASM")
    parser.add_argument("--dma", action="store_true",
                        help="execute DMA tile uploads and IRQ/scanout checkpoints in WASM")
    parser.add_argument("--pcm", action="store_true",
                        help="execute synchronized PCM and square checkpoints in WASM")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    manifest = root / "fixtures/manifest.toml"
    fixtures = tomllib.loads(manifest.read_text())["fixture"]
    names = ["cartridge"]
    if args.diagnostics:
        names.extend(["arm", "thumb", "services", "memory"])
    if args.vblank:
        names.append("vblank")
    if args.keypad:
        names.extend(["keypad-or", "keypad-and"])
    if args.dma:
        names.append("dma")
    if args.pcm:
        names.append("pcm")
    build = subprocess.run(
        ["cargo", "build", "--locked", "-p", "gba-core", "--release",
         "--target", "wasm32-unknown-unknown", "--message-format=json"],
        cwd=root, check=True, text=True, stdout=subprocess.PIPE,
    )
    artifacts = [json.loads(line) for line in build.stdout.splitlines()]
    library = next(
        filename for item in artifacts
        if item.get("reason") == "compiler-artifact"
        and item["target"]["name"] == "gba_core"
        for filename in item["filenames"] if filename.endswith(".rlib")
    )
    with tempfile.TemporaryDirectory(prefix="gba-wasm-timing-") as temporary:
        directory = Path(temporary)
        binary = directory / "timing.wasm"
        contract = directory / "contract.json"
        for name in names:
            fixture = next(item for item in fixtures if item["name"] == name)
            rom = (manifest.parent / fixture["rom"]).resolve()
            data = rom.read_bytes()
            if hashlib.sha256(data).hexdigest() != fixture["sha256"]:
                raise ValueError(f"{name} ROM differs from frozen SHA-256")
            verification = fixture["verification"]
            if verification["kind"] in ("diagnostic", "vblank", "keypad", "dma", "pcm"):
                firmware = root / "roms/test-firmware/division.bin"
                if hashlib.sha256(firmware.read_bytes()).hexdigest() != verification["firmware_sha256"]:
                    raise ValueError("test firmware differs from frozen SHA-256")
            if verification["kind"] == "diagnostic":
                offset = verification["terminal_pc"] - 0x08000000
                if data[offset:offset + 4] != verification["terminal_instruction"].to_bytes(4, "little"):
                    raise ValueError("diagnostic terminal opcode mismatch")
            contract.write_text(json.dumps(fixture))
            subprocess.run(
                ["rustc", "--edition=2024", "--crate-type=cdylib",
                 "--target=wasm32-unknown-unknown", "-C", "opt-level=3",
                 "-C", "panic=abort", "-D", "warnings", "--extern", f"gba_core={library}",
                 str(root / "roms/cartridge/wasm_harness.rs"), "-o", str(binary)],
                cwd=root, check=True, env={**os.environ, "GBA_VERIFICATION_ROM": str(rom)},
            )
            subprocess.run(
                ["node", str(root / "roms/cartridge/verify_wasm.mjs"),
                 str(binary), str(contract)], cwd=root, check=True,
            )


if __name__ == "__main__":
    main()
