#!/usr/bin/env python3
"""Check frozen ROM/WAV identities and execute native and production WASM contracts."""
import hashlib
import json
import tomllib
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[2]
fixtures = Path(__file__).resolve().parent
expected = tomllib.loads((fixtures / "manifest.toml").read_text())
assert hashlib.sha256((root / "roms/noise.gba").read_bytes()).hexdigest() == expected["sha256"]
with tempfile.TemporaryDirectory(prefix="gba-noise-recording-") as temporary:
    recording = Path(temporary) / "noise.wav"
    subprocess.run(["cargo", "run", "--locked", "-p", "gba-tools", "--release", "--", "verify-noise", "--capture", str(recording)], cwd=root, check=True)
    assert hashlib.sha256(recording.read_bytes()).hexdigest() == expected["wav_sha256"], "Recorded signal differs from frozen identity"
build = subprocess.run(["cargo", "build", "--locked", "-p", "gba-session", "--release", "--target", "wasm32-unknown-unknown", "--message-format=json"], cwd=root, check=True, text=True, stdout=subprocess.PIPE)
items = [json.loads(line) for line in build.stdout.splitlines()]
library = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_session" for file in item["filenames"] if file.endswith(".rlib"))
core = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_core" for file in item["filenames"] if file.endswith(".rlib"))
with tempfile.TemporaryDirectory(prefix="gba-noise-") as temporary:
    binary = str(Path(temporary) / "noise.wasm")
    subprocess.run(["rustc", "--edition=2024", "--crate-type=cdylib", "--target=wasm32-unknown-unknown", "-C", "opt-level=3", "-C", "panic=abort", "-D", "warnings", "--extern", "gba_session=" + library, "--extern", "gba_core=" + core, "-L", "dependency=" + str(root / "target/wasm32-unknown-unknown/release/deps"), str(fixtures / "wasm_harness.rs"), "-o", binary], cwd=root, check=True)
    subprocess.run(["node", "--input-type=module", "-e", "import fs from 'node:fs'; const {instance} = await WebAssembly.instantiate(fs.readFileSync(process.argv[1]), {}); if (instance.exports.verify() !== 1) throw Error('Noise contract failed'); console.log('WASM noise PASS: noise scene, four timer cascades, DMA1/DMA2 FIFOs, sustained mixing and chunk invariance');", binary], check=True)
