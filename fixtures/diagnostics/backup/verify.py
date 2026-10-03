#!/usr/bin/env python3
"""Check frozen cartridge hashes and execute the load contract in production WASM."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[3]
fixtures = Path(__file__).resolve().parent
for case in json.loads((fixtures / "manifest.json").read_text()):
    assert hashlib.sha256((fixtures / (case["name"] + ".gba")).read_bytes()).hexdigest() == case["sha256"], case["name"]
build = subprocess.run(["cargo", "build", "--locked", "-p", "gba-core", "--release", "--target", "wasm32-unknown-unknown", "--message-format=json"], cwd=root, check=True, text=True, stdout=subprocess.PIPE)
items = [json.loads(line) for line in build.stdout.splitlines()]
library = next(file for item in items if item.get("reason") == "compiler-artifact" and item["target"]["name"] == "gba_core" for file in item["filenames"] if file.endswith(".rlib"))
with tempfile.TemporaryDirectory(prefix="gba-backup-") as temporary:
    binary = str(Path(temporary) / "backup.wasm")
    subprocess.run(["rustc", "--edition=2024", "--crate-type=cdylib", "--target=wasm32-unknown-unknown", "-C", "opt-level=3", "-C", "panic=abort", "-D", "warnings", "--extern", "gba_core=" + library, str(fixtures / "wasm_harness.rs"), "-o", binary], cwd=root, check=True)
    subprocess.run(["node", "--input-type=module", "-e", "import fs from 'node:fs'; const {instance} = await WebAssembly.instantiate(fs.readFileSync(process.argv[1]), {}); if (instance.exports.verify() !== 1) throw Error('backup contract failed'); console.log('WASM backup contract PASS: 11 frozen cartridges, overrides and reset');", binary], check=True)
