'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const vm = require('node:vm');

// Execute the production processor with only its host boundary replaced. No
// browser, audio device, or duplicate queue implementation is needed.
function workletContext() {
    let Processor;
    const context = vm.createContext({
        Float32Array, ArrayBuffer, URL,
        AudioWorkletProcessor: class {
            constructor() { this.port = { postMessage() {} }; }
        },
        registerProcessor(_name, implementation) { Processor = implementation; },
    });
    vm.runInContext(fs.readFileSync(path.join(__dirname, 'pcm-worklet.js'), 'utf8'), context);
    return { context, create: options => new Processor(options) };
}

function render(processor, frames) {
    const channels = [new Float32Array(frames), new Float32Array(frames)];
    assert.equal(processor.process([], [channels]), true);
    return channels;
}

function send(processor, data) { processor.port.onmessage({ data }); }

function submit(processor, epoch, frames, value) {
    const samples = new Float32Array(frames * 2).fill(value);
    send(processor, { kind: 'pcm', epoch, frames, buffer: samples.buffer });
}

// Catches a short queue shortage turning into another startup silence. The
// Rust resampler and WASM guest contracts never execute this queue consumer.
test('transient underrun resumes fresh samples and lifecycle boundaries re-prime', () => {
    const processor = workletContext().create({ processorOptions: { capacity: 8, target: 4 } });
    send(processor, { kind: 'playing', value: true });
    send(processor, { kind: 'gain', value: 1 });
    submit(processor, 0, 4, 0.5);
    assert.deepEqual([...render(processor, 6)[0]], [0.5, 0.5, 0.5, 0.5, 0, 0]);
    submit(processor, 0, 1, 0.25);
    assert.deepEqual([...render(processor, 1)[0]], [0.25]);
    assert.equal(processor.underruns, 2);
    assert.equal(processor.underrunEvents, 1);
    assert.equal(processor.callbacks, 2);
    assert.equal(processor.maxCallbackFrames, 6);

    send(processor, { kind: 'clear', epoch: 1 });
    submit(processor, 0, 4, 0.75);
    submit(processor, 1, 1, 0.125);
    assert.deepEqual([...render(processor, 1)[0]], [0]);
    submit(processor, 1, 3, 0.125);
    assert.deepEqual([...render(processor, 1)[0]], [0.125]);
    submit(processor, 1, 12, 0.125);
    assert.equal(processor.depth, 8);
    assert.equal(processor.overflows, 7);
    assert.equal(processor.overflowEvents, 1);
});

// Exercise the actual bridge configuration: buffering 40 ms must be enough
// to start playback. Testing the processor alone cannot catch a stale 60 ms
// threshold passed by the main thread.
test('browser bridge starts playback with 40 ms of queued samples', async () => {
    const host = workletContext();
    let processor;
    Object.assign(host.context, {
        document: { baseURI: 'http://localhost/', addEventListener() {} },
        window: { addEventListener() {} },
        setInterval() { return 1; }, clearInterval() {},
        AudioContext: class {
            constructor() {
                this.sampleRate = 48000;
                this.state = 'running';
                this.audioWorklet = { async addModule() {} };
            }
            async resume() {}
        },
        AudioWorkletNode: class {
            constructor(_context, _name, options) {
                processor = host.create(options);
                this.port = { postMessage: data => send(processor, data) };
                processor.port.postMessage = data => this.port.onmessage?.({ data });
            }
            connect() {}
        },
    });
    vm.runInContext(fs.readFileSync(path.join(__dirname, 'audio.js'), 'utf8'), host.context);
    host.context.gbaAudio.start();
    // Drain the asynchronous module-loading continuation without a timed wait.
    await new Promise(resolve => setImmediate(resolve));
    host.context.gbaAudio.setPlaying(true);
    host.context.gbaAudio.setGain(1);
    host.context.gbaAudio.submit(new Float32Array(1920 * 2).fill(0.25));
    assert.deepEqual([...render(processor, 1)[0]], [0.25]);
    send(processor, { kind: 'stats' });
    const status = host.context.gbaAudio.status();
    assert.match(status, /underrun events 0 frames 0/);
    assert.match(status, /overflow events 0 frames 0/);
    assert.match(status, /callbacks 1 max 1 frames/);
});
