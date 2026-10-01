#!/usr/bin/env python3
"""Compile the production BIOS guest contract to WASM and run a supplied BIOS."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--bios', type=Path)
parser.add_argument('--expected-frame-sha256', default='59ce42abae9825c2d2579c5cd838e47d88be917e37ea36ff162d46fc5d0991e3',
                    help='override only for failure/mutation validation')
parser.add_argument('--build-only', action='store_true', help='compile without running private firmware')
args = parser.parse_args()
if not args.build_only and args.bios is None:
    parser.error('--bios is required unless --build-only is selected')
root = Path(__file__).resolve().parents[2]
rom = (root / 'roms/gba-tests/bios/bios.gba').read_bytes()
if hashlib.sha256(rom).hexdigest() != '9d7b369fa1aa661ff03692b3d79c6f644b623d72983d0fc890e6d87a0409a3c9':
    raise ValueError('bios.gba differs from pinned identity')
build = subprocess.run(['cargo', 'build', '--locked', '-p', 'gba-core', '--release',
                        '--target', 'wasm32-unknown-unknown', '--message-format=json'],
                       cwd=root, check=True, text=True, stdout=subprocess.PIPE)
artifacts = [json.loads(line) for line in build.stdout.splitlines()]
library = next(filename for item in artifacts
               if item.get('reason') == 'compiler-artifact' and item['target']['name'] == 'gba_core'
               for filename in item['filenames'] if filename.endswith('.rlib'))
with tempfile.TemporaryDirectory(prefix='gba-bios-wasm-') as temporary:
    binary = Path(temporary) / 'bios.wasm'
    subprocess.run(['rustc', '--edition=2024', '--crate-type=cdylib',
                    '--target=wasm32-unknown-unknown', '-C', 'opt-level=3', '-C', 'panic=abort',
                    '-D', 'warnings', '--extern', f'gba_core={library}',
                    str(root / 'roms/bios/wasm_harness.rs'), '-o', str(binary)], cwd=root, check=True)
    if args.build_only:
        print('PASS compiled BIOS guest contract for WASM (execution pending)')
        raise SystemExit(0)
    subprocess.run(['node', str(root / 'roms/bios/verify_wasm.mjs'),
                    str(binary), str(args.bios.resolve()), args.expected_frame_sha256], cwd=root, check=True)
