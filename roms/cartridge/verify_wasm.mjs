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
if (verification.kind === "keypad") {
    const mailbox = fixture.mailbox_address;
    const wake = verification.and_mode ? 30000n : 20000n;
    const advance = target => core.advance(target, fixture.max_instructions, BigInt(fixture.max_cycles));
    core.initialize(1);
    core.queue_keypad_input();
    advance(wake - 1n);
    assert.equal(core.inspect16(0x04000202), 0, "unrelated/incomplete keys must not request IF");
    assert.equal(core.inspect16(mailbox + 2), 0, "no premature callback");
    assert.equal(core.inspect16(mailbox + 8), 0, "no premature wake");
    assert.equal(core.halted(), 1);
    advance(wake);
    assert.equal(core.cycles(), wake, "HALT must stop at the exact shared input deadline");
    assert.equal(core.inspect16(0x04000202), 0x1000, "keypad source");
    assert.equal(core.inspect16(mailbox + 2), 0, "callback runs after request boundary");
    for (let frame = 1; frame <= 2; frame++) {
        advance(BigInt(frame) * 280896n);
        assert.equal(core.generation(), BigInt(frame), "completed frame boundary");
        assert.equal(core.inspect16(mailbox), fixture.completion_id);
        assert.equal(core.inspect16(mailbox + 2), 1, "one callback");
        assert.equal(core.inspect16(mailbox + 8), 1, "one wake");
        assert.equal(core.inspect16(mailbox + 4), 0x1000, "saved pending IF");
        assert.equal(core.inspect16(mailbox + 6), 0, "saved W1C result");
        assert.equal(core.inspect16(0x04000202), 0, "acknowledged IF");
        assert.equal(core.halted(), 1);
        // Frame one started before the IRQ. Frame two must be entirely red.
        if (frame === 2) {
            for (let index = 0; index < 240 * 160; index++) {
                assert.equal(core.pixel(index), 0x001f, `bright red guest frame ${frame}, pixel ${index}`);
            }
        }
    }
    const instructions = core.instructions();
    core.initialize_variant(0x300, verification.and_mode ? 0x8003 : 3, 1);
    core.queue_keypad_input();
    advance(2n * 280896n);
    assert.equal(core.inspect16(0x04000202), 0, "disabled source IF");
    assert.equal(core.inspect16(mailbox + 2), 0, "disabled source callback");
    assert.equal(core.inspect16(mailbox + 8), 0, "disabled source wake");
    assert.equal(core.halted(), 1);
    core.initialize(0);
    core.queue_keypad_input();
    assert.equal(core.rejects_advance(280896n, fixture.max_instructions), 1);
    assert.equal(core.inspect16(mailbox + 2), 0, "unmapped vector cannot dispatch callback");
    console.log(`PASS WASM ${fixture.name} wake_cycle=${wake} frames=2 pixels=38400 instructions=${instructions} IF=0x1000 acknowledgement=0 disabled-stall unmapped-vector-rejected`);
    process.exit(0);
}
if (verification.kind === "vblank") {
    const mailbox = fixture.mailbox_address;
    for (const configuration of [7, 15, 6, 5, 3]) {
        core.initialize_variant(verification.configuration_offset, configuration, 1);
        for (let frame = 1; frame <= verification.frames; frame++) {
            core.advance(BigInt(frame) * 280896n, fixture.max_instructions, BigInt(fixture.max_cycles));
            const callbacks = configuration === 7 || configuration === 15 ? frame : 0;
            assert.equal(core.inspect16(mailbox), fixture.completion_id);
            assert.equal(core.inspect16(mailbox + 2), callbacks);
            assert.equal(core.inspect16(mailbox + 8), configuration === 6 ? 0 : frame);
            assert.equal(core.halted(), 1);
            if (callbacks) {
                assert.equal(core.inspect16(mailbox + 4), 1);
                assert.equal(core.inspect16(mailbox + 6), 0);
                assert.equal(core.cpsr() & 0xff, configuration === 15 ? 0x7f : 0x5f);
            }
            for (let index = 0; index < 240 * 160; index++) {
                assert.equal(core.pixel(index), callbacks ? (frame - 1) & 31 : 0);
            }
        }
        console.log(`PASS WASM vblank configuration=${configuration} frames=${verification.frames} instructions=${core.instructions()} ${configuration === 6 ? "expected-stall" : "wake/dispatch"}`);
    }
    core.initialize(0);
    assert.equal(core.rejects_advance(280896n, fixture.max_instructions), 1);
    assert.equal(core.inspect16(mailbox + 2), 0);
    console.log("PASS WASM vblank unmapped vector rejected before callback");
    process.exit(0);
}
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
