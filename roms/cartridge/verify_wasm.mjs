// Execute compiled WASM with no native Rust fallback. All completion and timing
// expectations come from the manifest parsed by the Python driver.
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";

const [binary, contract] = process.argv.slice(2);
const fixture = JSON.parse(readFileSync(contract, "utf8"));
const module = await WebAssembly.compile(readFileSync(binary));
assert.deepEqual(WebAssembly.Module.imports(module), [], "unexpected host dependency");
const { exports: core } = await WebAssembly.instantiate(module, {});
const verification = fixture.verification;
const marker = (pc) => core.marker(pc, fixture.max_instructions, BigInt(fixture.max_cycles));
if (verification.kind === "diagnostic") {
    core.initialize(1);
    const total = marker(verification.terminal_pc);
    const instructions = core.instructions();
    assert.equal(core.register(verification.result_register) >>> 0, verification.result, "diagnostic result");
    assert.equal((core.cpsr() & verification.cpsr_mask) >>> 0, verification.cpsr_value, "diagnostic CPSR");
    if (verification.framebuffer_sha256) {
        // Match the native runner's subsequent complete-scanout boundary.
        const frameCycles = 280896n;
        const target = (total / frameCycles + 2n) * frameCycles;
        core.advance(target, fixture.max_instructions, BigInt(fixture.max_cycles));
        const image = Buffer.alloc(240 * 160 * 2);
        for (let index = 0; index < 240 * 160; index++) {
            image.writeUInt16LE(core.pixel(index), index * 2);
        }
        assert.equal(createHash("sha256").update(image).digest("hex"), verification.framebuffer_sha256, "diagnostic success screen");
    }
    console.log(`PASS WASM ${fixture.name} terminal=0x${verification.terminal_pc.toString(16)} cycles=${total} instructions=${instructions} result=${verification.result} success_screen=${Boolean(verification.framebuffer_sha256)}`);
    process.exit(0);
}
const points = verification.checkpoints;
assert.equal(points.length, 24, "all timing cases must execute");
core.initialize(0);
for (const point of points) {
    const start = marker(point.start_pc);
    assert.equal(core.inspect16(0x04000204), point.waitcnt, "start WAITCNT");
    const elapsed = marker(point.end_pc) - start;
    assert.equal(core.inspect16(0x04000204), point.waitcnt, "end WAITCNT");
    assert.equal(elapsed, BigInt(point.cycles), `WS${point.window}/${point.width} timing`);
    console.log(`PASS WASM WS${point.window} width=${point.width} WAITCNT=0x${point.waitcnt.toString(16).padStart(4, "0")} cycles=${elapsed}`);
}
const total = marker(verification.terminal_pc);
const mailbox = fixture.mailbox_address;
assert.equal(core.inspect16(mailbox), fixture.completion_id, "completion identity");
assert.equal(core.inspect16(mailbox + 2), 1, "completion result");
assert.equal(core.inspect16(mailbox + 4), 0x5678, "data low halfword");
assert.equal(core.inspect16(mailbox + 6), 0x1234, "data high halfword");
points.forEach((point, index) => {
    assert.equal(core.inspect16(mailbox + 8 + index * 2), point.cycles, "guest cycle report");
});
console.log(`PASS WASM terminal=0x${verification.terminal_pc.toString(16)} mailbox=0x${fixture.completion_id.toString(16)} cycles=${total} limits=${fixture.max_instructions}/${fixture.max_cycles}`);
