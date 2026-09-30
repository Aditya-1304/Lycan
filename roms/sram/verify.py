#!/usr/bin/env python3
"""Check frozen cartridge hashes and execute the load contract in production WASM."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
fixtures = Path(__file__).resolve().parent
case = json.loads((fixtures / "manifest.json").read_text())
assert hashlib.sha256((root / "roms/sram.gba").read_bytes()).hexdigest() == case["sha256"]
build = subprocess.run(["cargo", "build", "--locked", "-p", "gba-session", "--release", "--target", "wasm32-unknown-unknown", "--message-format=json"], cwd=root, check=True, text=True, stdout=subprocess.PIPE)
items = [json.loads(line) for line in build.stdout.splitlines()]
library = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_session" for file in item["filenames"] if file.endswith(".rlib"))
with tempfile.TemporaryDirectory(prefix="gba-sram-") as temporary:
    binary = str(Path(temporary) / "sram.wasm")
    subprocess.run(["rustc", "--edition=2024", "--crate-type=cdylib", "--target=wasm32-unknown-unknown", "-C", "opt-level=3", "-C", "panic=abort", "-D", "warnings", "--extern", "gba_session=" + library, "-L", "dependency=" + str(root / "target/wasm32-unknown-unknown/release/deps"), str(fixtures / "wasm_harness.rs"), "-o", binary], cwd=root, check=True)
    subprocess.run(["node", "--input-type=module", "-e", "import fs from 'node:fs'; const {instance} = await WebAssembly.instantiate(fs.readFileSync(process.argv[1]), {}); if (instance.exports.verify() !== 1) throw Error('sram contract failed'); console.log('WASM sram contract PASS: guest score, restore, revisions and image validation');", binary], check=True)
