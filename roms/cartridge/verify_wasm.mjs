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
if (verification.kind === "pcm") {
    core.initialize(1);
    const setup = marker(verification.ready_pc);
    assert.equal(core.inspect16(fixture.mailbox_address), fixture.completion_id);
    core.queue_pcm_input();
    const last = verification.checkpoints.at(-1).frame;
    let total = 0;
    for (let frame = 1; frame <= last; frame++) {
        core.advance(BigInt(frame) * 280896n, fixture.max_instructions, BigInt(fixture.max_cycles));
        const count = core.drain_pcm();
        total += count;
        const point = verification.checkpoints.find(point => point.frame === frame);
        if (!point) continue;
        const pcm = Buffer.alloc(count);
        for (let i = 0; i < count; i++) pcm[i] = core.pcm_byte(i);
        assert.equal(count, point.samples);
        assert.equal(createHash("sha256").update(pcm).digest("hex"), point.pcm_sha256);
        for (let index = 0; index < 38400; index++) {
            const x = index % 240, y = Math.floor(index / 240);
            assert.equal(core.pixel(index), x >= 112 && x < 128 && y >= 72 && y < 88 ? point.color : 0);
        }
        assert.equal(core.inspect16(fixture.mailbox_address + 2), frame);
        assert.equal(core.inspect16(fixture.mailbox_address + 4), point.pressed);
        assert.equal(core.inspect16(fixture.mailbox_address + 6), point.transitions);
        assert.equal(core.inspect16(0x04000202), 0);
        assert.equal(core.halted(), 1);
        console.log(`PASS WASM pcm frame=${frame} samples=${count} square=${point.color} transitions=${point.transitions} pixels=38400`);
    }
    assert.equal(core.pcm_counter(0), BigInt(total));
    assert.equal(core.pcm_counter(0), core.cycles() / 512n);
    assert.equal(core.pcm_counter(1), 0n);
    assert.equal(core.pcm_counter(2), 0n);
    console.log(`PASS WASM pcm setup_cycles=${setup} cycles=${core.cycles()} instructions=${core.instructions()} produced=${total} rate=32768`);
    core.queue_mixer_input();
    for (let frame = last + 1; frame <= verification.mixer_checkpoints.at(-1).frame; frame++) {
        core.advance(BigInt(frame) * 280896n, fixture.max_instructions, BigInt(fixture.max_cycles));
        const count = core.drain_stereo_pcm();
        total += count;
        const point = verification.mixer_checkpoints.find(point => point.frame === frame);
        if (!point) continue;
        const pcm = Buffer.alloc(count * 4);
        const observed = new Set();
        for (let i = 0; i < count; i++) {
            const pair = [core.stereo_level(i, 0), core.stereo_level(i, 1)];
            observed.add(JSON.stringify(pair));
            assert(point.levels.some(level => level[0] === pair[0] && level[1] === pair[1]));
            pcm.writeInt16LE(pair[0], i * 4);
            pcm.writeInt16LE(pair[1], i * 4 + 2);
        }
        for (const pair of point.levels) assert(observed.has(JSON.stringify(pair)));
        assert.equal(createHash("sha256").update(pcm).digest("hex"), point.pcm_sha256);
        assert.equal(core.inspect16(0x04000082), point.control_h);
        assert.equal(core.inspect16(0x04000084), point.master);
        assert.equal(core.inspect16(0x04000088), point.bias);
        assert.equal(core.inspect16(fixture.mailbox_address + 2), frame);
        assert.equal(core.inspect16(fixture.mailbox_address + 4), 1);
        assert.equal(core.inspect16(fixture.mailbox_address + 6), 3);
        assert.equal(core.halted(), 1);
        console.log(`PASS WASM mixer frame=${frame} samples=${count} sha256=${point.pcm_sha256}`);
    }
    assert.equal(core.pcm_counter(0), BigInt(total));
    assert.equal(core.pcm_counter(0), core.cycles() / 512n);
    assert.equal(core.pcm_counter(1), 0n);
    assert.equal(core.pcm_counter(2), BigInt(verification.mixer_fifo_underruns));
    console.log(`PASS WASM mixer cycles=${core.cycles()} instructions=${core.instructions()} produced=${total} fifo_underruns=${core.pcm_counter(2)}`);
    process.exit(0);
}
if (verification.kind === "dma") {
    core.initialize(1);
    const setup = marker(verification.ready_pc);
    assert.equal(core.inspect16(fixture.mailbox_address), fixture.completion_id);
    for (let address = 0x06000000; address < 0x06010000; address += 2) {
        assert.equal(core.inspect16(address), address >= 0x06008000 && address < 0x0600a000 ? 0 : 0x1111, "large DMA upload and map clear");
    }
    core.queue_dma_input();
    for (const point of verification.checkpoints) {
        core.advance(BigInt(point.frame) * 280896n, fixture.max_instructions, BigInt(fixture.max_cycles));
        for (let address = 0x06000000; address < 0x06000020; address += 2) {
            assert.equal(core.inspect16(address), point.tile, "uploaded tile data");
        }
        for (let index = 0; index < 38400; index++) {
            assert.equal(core.pixel(index), point.color, `DMA frame ${point.frame} pixel ${index}`);
        }
        assert.equal(core.inspect16(fixture.mailbox_address + 2), point.vblank);
        assert.equal(core.inspect16(fixture.mailbox_address + 4), point.completions);
        assert.equal(core.inspect16(fixture.mailbox_address + 6), 0x801);
        assert.equal(core.inspect16(fixture.mailbox_address + 8), 0);
        assert.equal(core.halted(), 1);
        console.log(`PASS WASM dma frame=${point.frame} pixels=38400 vblank=${point.vblank} completions=${point.completions}`);
    }
    console.log(`PASS WASM dma setup_cycles=${setup} cycles=${core.cycles()} instructions=${core.instructions()}`);
    process.exit(0);
}
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
