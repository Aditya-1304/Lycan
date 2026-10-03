#!/usr/bin/env python3
"""Verify pinned ROMs, captures, upstream geometry, and production WASM execution."""
import hashlib
import json
import struct
import tomllib
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[3]
fixtures = Path(__file__).resolve().parent
expected = tomllib.loads((fixtures / "manifest.toml").read_text())
for name, identity in [("affine-object", expected), ("affine-object-degenerate", expected["mgba_degenerate"])]:
    data = (root / f"fixtures/diagnostics/{name}.gba").read_bytes()
    assert len(data) == identity["bytes"]
    assert hashlib.sha256(data).hexdigest() == identity["sha256"]
# Crop the upstream 256x128 image to LCD width; extend its white backdrop below.
bmp = (fixtures / "mgba-expected.bmp").read_bytes()
assert hashlib.sha256(bmp).hexdigest() == expected["mgba_degenerate"]["expected_bmp_sha256"]
assert struct.unpack_from("<IiiHH", bmp, 14) == (40, 256, 128, 1, 4)
offset = struct.unpack_from("<I", bmp, 10)[0]
image = bytearray()
for y in range(160):
    for x in range(240):
        color = 0x7fff
        if y < 128:
            packed = bmp[offset + (127-y)*128 + x//2]
            index = packed >> 4 if x % 2 == 0 else packed & 15
            blue, green, red, _ = bmp[54+index*4:58+index*4]
            color = (red >> 3) | ((green >> 3) << 5) | ((blue >> 3) << 10)
        image.extend(struct.pack("<H", color))
assert image == (fixtures / "mgba-expected.bin").read_bytes()
assert hashlib.sha256(image).hexdigest() == expected["mgba_degenerate"]["expected_bgr555_sha256"]
subprocess.run(["cargo", "run", "--locked", "-p", "gba-tools", "--release", "--", "verify-affine-object", "--capture", str(root / "target/affine-object-captures")], cwd=root, check=True)
for name, digest in json.loads((fixtures / "captures.json").read_text()).items():
    assert hashlib.sha256((root / "target/affine-object-captures" / name).read_bytes()).hexdigest() == digest, name
build = subprocess.run(["cargo", "build", "--locked", "-p", "gba-session", "--release", "--target", "wasm32-unknown-unknown", "--message-format=json"], cwd=root, check=True, text=True, stdout=subprocess.PIPE)
items = [json.loads(line) for line in build.stdout.splitlines()]
library = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_session" for file in item["filenames"] if file.endswith(".rlib"))
core = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_core" for file in item["filenames"] if file.endswith(".rlib"))
with tempfile.TemporaryDirectory(prefix="gba-object-") as temporary:
    binary = str(Path(temporary) / "object.wasm")
    subprocess.run(["rustc", "--edition=2024", "--crate-type=cdylib", "--target=wasm32-unknown-unknown", "-C", "opt-level=3", "-C", "panic=abort", "-D", "warnings", "--extern", "gba_session=" + library, "--extern", "gba_core=" + core, "-L", "dependency=" + str(root / "target/wasm32-unknown-unknown/release/deps"), str(fixtures / "wasm_harness.rs"), "-o", binary], cwd=root, check=True)
    subprocess.run(["node", "--input-type=module", "-e", "import fs from 'node:fs'; const {instance} = await WebAssembly.instantiate(fs.readFileSync(process.argv[1]), {}); if (instance.exports.verify() !== 1) throw Error('Affine object contract failed'); console.log('WASM affine-object PASS: 14 interactive captures and mGBA singular-matrix comparison');", binary], check=True)
