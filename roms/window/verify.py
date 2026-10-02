#!/usr/bin/env python3
"""Verify frozen guest/capture identities and native/production WASM replay."""
import hashlib
import json
import tomllib
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[2]
fixtures = Path(__file__).resolve().parent
expected = tomllib.loads((fixtures / "manifest.toml").read_text())
data = (root / "roms/window.gba").read_bytes()
assert expected["mailbox_id"] == 0xa4
assert expected["mailbox_address"] == 0x03000000
assert (expected["polling_pc_start"], expected["polling_pc_end"]) == (0x080001b0, 0x080001c4)
assert expected["instruction_limit"] == 2_000_000
assert expected["cycle_overshoot_limit"] == 32
assert expected["capture_frames"] == list(range(12, 53, 4))
assert len(data) == expected["bytes"]
assert hashlib.sha256(data).hexdigest() == expected["sha256"]
subprocess.run(["cargo", "run", "--locked", "-p", "gba-tools", "--release", "--", "verify-window"], cwd=root, check=True)
for name, digest in json.loads((fixtures / "captures.json").read_text()).items():
    assert hashlib.sha256((root / "target/window-captures" / name).read_bytes()).hexdigest() == digest, name
build = subprocess.run(["cargo", "build", "--locked", "-p", "gba-session", "--release", "--target", "wasm32-unknown-unknown", "--message-format=json"], cwd=root, check=True, text=True, stdout=subprocess.PIPE)
items = [json.loads(line) for line in build.stdout.splitlines()]
library = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_session" for file in item["filenames"] if file.endswith(".rlib"))
core = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_core" for file in item["filenames"] if file.endswith(".rlib"))
with tempfile.TemporaryDirectory(prefix="gba-window-") as temporary:
    binary = str(Path(temporary) / "window.wasm")
    subprocess.run(["rustc", "--edition=2024", "--crate-type=cdylib", "--target=wasm32-unknown-unknown", "-C", "opt-level=3", "-C", "panic=abort", "-D", "warnings", "--extern", "gba_session=" + library, "--extern", "gba_core=" + core, "-L", "dependency=" + str(root / "target/wasm32-unknown-unknown/release/deps"), str(fixtures / "wasm_harness.rs"), "-o", binary], cwd=root, check=True)
    subprocess.run(["node", "--input-type=module", "-e", "import fs from 'node:fs'; const {instance} = await WebAssembly.instantiate(fs.readFileSync(process.argv[1]), {}); if (instance.exports.verify() !== 1) throw Error('Window contract failed'); console.log('WASM window PASS: 11 interactive full-frame captures');", binary], check=True)
