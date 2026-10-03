#!/usr/bin/env python3
"""Check frozen ROM/WAV identities and execute native and production WASM contracts."""
import hashlib
import json
import tomllib
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[3]
fixtures = Path(__file__).resolve().parent
expected = tomllib.loads((fixtures / "manifest.toml").read_text())
assert hashlib.sha256((root / "fixtures/diagnostics/pulse.gba").read_bytes()).hexdigest() == expected["sha256"]
with tempfile.TemporaryDirectory(prefix="gba-pulse-recording-") as temporary:
    recording = Path(temporary) / "pulse.wav"
    subprocess.run(["cargo", "run", "--locked", "-p", "gba-tools", "--release", "--", "verify-pulse", "--capture", str(recording)], cwd=root, check=True)
    assert hashlib.sha256(recording.read_bytes()).hexdigest() == expected["wav_sha256"], "Recorded signal differs from frozen identity"
build = subprocess.run(["cargo", "build", "--locked", "-p", "gba-session", "--release", "--target", "wasm32-unknown-unknown", "--message-format=json"], cwd=root, check=True, text=True, stdout=subprocess.PIPE)
items = [json.loads(line) for line in build.stdout.splitlines()]
library = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_session" for file in item["filenames"] if file.endswith(".rlib"))
core = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_core" for file in item["filenames"] if file.endswith(".rlib"))
with tempfile.TemporaryDirectory(prefix="gba-pulse-") as temporary:
    binary = str(Path(temporary) / "pulse.wasm")
    subprocess.run(["rustc", "--edition=2024", "--crate-type=cdylib", "--target=wasm32-unknown-unknown", "-C", "opt-level=3", "-C", "panic=abort", "-D", "warnings", "--extern", "gba_session=" + library, "--extern", "gba_core=" + core, "-L", "dependency=" + str(root / "target/wasm32-unknown-unknown/release/deps"), str(fixtures / "wasm_harness.rs"), "-o", binary], cwd=root, check=True)
    subprocess.run(["node", "--input-type=module", "-e", "import fs from 'node:fs'; const {instance} = await WebAssembly.instantiate(fs.readFileSync(process.argv[1]), {}); if (instance.exports.verify() !== 1) throw Error('Pulse contract failed'); console.log('WASM pulse PASS: notes, stereo voices, sweep/envelope/duty, mute/restart, FIFO mixing and chunk invariance');", binary], check=True)
