#!/usr/bin/env python3
"""Verify frozen identities, native captures, and production WASM guest execution."""
import hashlib
import json
import tomllib
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[2]
fixtures = Path(__file__).resolve().parent
expected = tomllib.loads((fixtures / "manifest.toml").read_text())
for mode in range(1, 6):
    data = (root / f"roms/affine-mode-{mode}.gba").read_bytes()
    assert hashlib.sha256(data).hexdigest() == expected[f"mode_{mode}"]["sha256"]
subprocess.run(["cargo", "run", "--locked", "-p", "gba-tools", "--release", "--", "verify-affine", "--capture", str(root / "target/affine-captures")], cwd=root, check=True)
# Full-frame source-geometry assertions run before comparing frozen capture files.
for name, digest in json.loads((fixtures / "captures.json").read_text()).items():
    assert hashlib.sha256((root / "target/affine-captures" / name).read_bytes()).hexdigest() == digest, name
build = subprocess.run(["cargo", "build", "--locked", "-p", "gba-session", "--release", "--target", "wasm32-unknown-unknown", "--message-format=json"], cwd=root, check=True, text=True, stdout=subprocess.PIPE)
items = [json.loads(line) for line in build.stdout.splitlines()]
library = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_session" for file in item["filenames"] if file.endswith(".rlib"))
core = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_core" for file in item["filenames"] if file.endswith(".rlib"))
with tempfile.TemporaryDirectory(prefix="gba-affine-") as temporary:
    binary = str(Path(temporary) / "affine.wasm")
    subprocess.run(["rustc", "--edition=2024", "--crate-type=cdylib", "--target=wasm32-unknown-unknown", "-C", "opt-level=3", "-C", "panic=abort", "-D", "warnings", "--extern", "gba_session=" + library, "--extern", "gba_core=" + core, "-L", "dependency=" + str(root / "target/wasm32-unknown-unknown/release/deps"), str(fixtures / "wasm_harness.rs"), "-o", binary], cwd=root, check=True)
    subprocess.run(["node", "--input-type=module", "-e", "import fs from 'node:fs'; const {instance} = await WebAssembly.instantiate(fs.readFileSync(process.argv[1]), {}); if (instance.exports.verify() !== 1) throw Error('Affine contract failed'); console.log('WASM affine PASS: 35 full-frame gradient captures in modes 1-5');", binary], check=True)
