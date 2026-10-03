#!/usr/bin/env python3

"""Verify frozen Slice-24 ROM identity plus native and production-WASM replay."""

from pathlib import Path
import hashlib
import json
import subprocess
import tempfile
import tomllib

root = Path(__file__).resolve().parents[3]
fixtures = Path(__file__).resolve().parent

expected = tomllib.loads(
    (fixtures / "manifest.toml").read_text()
)

if expected["bytes"] <= 0:
    raise SystemExit(
        "Freeze manifest bytes from `python3 fixtures/diagnostics/blend/build.py` first"
    )

if expected["sha256"] == "FREEZE_AFTER_FIRST_BUILD":
    raise SystemExit(
        "Freeze manifest SHA-256 from `python3 fixtures/diagnostics/blend/build.py` first"
    )

data = (root / "fixtures/diagnostics/blend.gba").read_bytes()

assert expected["mailbox_id"] == 0xA5
assert expected["mailbox_address"] == 0x03000000
assert expected["instruction_limit"] == 2_000_000
assert expected["cycle_overshoot_limit"] == 32
assert expected["capture_frames"] == list(range(12, 41, 4))

assert len(data) == expected["bytes"]

assert hashlib.sha256(data).hexdigest() == expected["sha256"]

subprocess.run(
    [
        "cargo",
        "run",
        "--locked",
        "-p",
        "gba-tools",
        "--release",
        "--",
        "verify-blend",
    ],
    cwd=root,
    check=True,
)

build = subprocess.run(
    [
        "cargo",
        "build",
        "--locked",
        "-p",
        "gba-session",
        "--release",
        "--target",
        "wasm32-unknown-unknown",
        "--message-format=json",
    ],
    cwd=root,
    check=True,
    text=True,
    stdout=subprocess.PIPE,
)

items = [
    json.loads(line)
    for line in build.stdout.splitlines()
]

library = next(
    file
    for item in items
    if item.get("reason") == "compiler-artifact"
    and item["target"]["name"] == "gba_session"
    for file in item["filenames"]
    if file.endswith(".rlib")
)

core = next(
    file
    for item in items
    if item.get("reason") == "compiler-artifact"
    and item["target"]["name"] == "gba_core"
    for file in item["filenames"]
    if file.endswith(".rlib")
)

with tempfile.TemporaryDirectory(prefix="gba-blend-") as temporary:
    binary = str(Path(temporary) / "blend.wasm")

    subprocess.run(
        [
            "rustc",
            "--edition=2024",
            "--crate-type=cdylib",
            "--target=wasm32-unknown-unknown",
            "-C",
            "opt-level=3",
            "-C",
            "panic=abort",
            "-D",
            "warnings",
            "--extern",
            "gba_session=" + library,
            "--extern",
            "gba_core=" + core,
            "-L",
            "dependency="
            + str(
                root
                / "target/wasm32-unknown-unknown/release/deps"
            ),
            str(fixtures / "harness.rs"),
            "-o",
            binary,
        ],
        cwd=root,
        check=True,
    )

    subprocess.run(
        [
            "node",
            "--input-type=module",
            "-e",
            (
                "import fs from 'node:fs';"
                "const {instance}=await WebAssembly.instantiate("
                "fs.readFileSync(process.argv[1]),{});"
                "if(instance.exports.verify()!==1)"
                "throw Error('Blend contract failed');"
                "console.log('WASM blend PASS: 8 full-frame captures');"
            ),
            binary,
        ],
        check=True,
    )
