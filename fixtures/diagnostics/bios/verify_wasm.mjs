// Execute supplied BIOS bytes in the compiled production WASM core.
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
const [binary, firmware, expected = '59ce42abae9825c2d2579c5cd838e47d88be917e37ea36ff162d46fc5d0991e3'] = process.argv.slice(2);
const bios = readFileSync(firmware);
assert.equal(bios.length, 16384, 'BIOS must be exactly 16 KiB');
const module = await WebAssembly.compile(readFileSync(binary));
assert.deepEqual(WebAssembly.Module.imports(module), [], 'unexpected host dependency');
const { exports: core } = await WebAssembly.instantiate(module, {});
bios.forEach((value, index) => core.bios_byte(index, value));
core.verify();
const image = Buffer.alloc(38400 * 2);
for (let index = 0; index < 38400; index++) image.writeUInt16LE(core.pixel(index), index * 2);
assert.equal(createHash('sha256').update(image).digest('hex'),
    expected,
    'upstream all-tests-passed framebuffer');
console.log(`PASS WASM bios.gba BIOS SHA-256=${createHash('sha256').update(bios).digest('hex')}`);
