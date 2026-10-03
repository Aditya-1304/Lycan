'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const test = require('node:test');

// Exercise the production bridge against an asynchronous device boundary. These
// cases catch queue loss and false readiness that processor-only tests cannot.
function bridge() {
    let device, processor, failModule = false, modules = 0;
    const messages = [];
    const host = vm.createContext({
        URL, Float32Array, ArrayBuffer,
        document: { baseURI: 'http://localhost/', addEventListener() {} },
        window: { addEventListener() {} },
        setInterval() { return 1; }, clearInterval() {},
        AudioContext: class {
            constructor() {
                device = this;
                this.state = 'suspended';
                this.sampleRate = 48000;
                this.audioWorklet = { async addModule() {
                    modules++;
                    if (failModule) throw new Error('module unavailable');
                } };
            }
            resume() { this.state = 'running'; return Promise.resolve(); }
            addEventListener(name, callback) { this[name] = callback; }
        },
        AudioWorkletNode: class {
            constructor() { processor = this; this.port = { postMessage: data => messages.push(data) }; }
            connect() {}
        },
    });
    vm.runInContext(fs.readFileSync(`${__dirname}/audio.js`, 'utf8'), host);
    return { audio: host.gbaAudio, messages, device: () => device, processor: () => processor,
        fail(value) { failModule = value; }, modules: () => modules };
}
const settle = () => new Promise(resolve => setImmediate(resolve));

test('healthy unlock preserves queued PCM, while suspension resumes the same node', async () => {
    const b = bridge();
    b.audio.start();
    assert.equal(b.audio.state(), 'starting');
    await settle();
    b.audio.setPlaying(true);
    b.messages.length = 0;
    b.audio.start();
    await settle();
    assert.equal(b.messages.filter(m => m.kind === 'clear').length, 0);
    b.device().state = 'suspended';
    b.device().statechange();
    // Context suspension can happen without a page blur; stale PCM must be dropped.
    assert.equal(b.messages.filter(m => m.kind === 'clear').length, 1);
    assert.equal(b.audio.state(), 'interaction');
    const processor = b.processor();
    b.audio.start();
    await settle();
    assert.equal(b.processor(), processor);
    assert.equal(b.audio.state(), 'ready');
    assert.equal(b.modules(), 1);
});

test('failed initialization stays visible until explicit retry and restores muted gain', async () => {
    const b = bridge();
    b.audio.setGain(0);
    b.fail(true);
    b.audio.start();
    await settle();
    assert.equal(b.audio.state(), 'unavailable');
    assert.match(b.audio.failure(), /module unavailable/);
    b.fail(false);
    b.audio.start();
    await settle();
    assert.equal(b.modules(), 1);
    b.audio.start(true);
    await settle();
    assert.equal(b.audio.state(), 'ready');
    assert.equal(b.messages.findLast(m => m.kind === 'gain').value, 0);
    b.processor().onprocessorerror();
    b.audio.start(true);
    await settle();
    assert.equal(b.audio.state(), 'unavailable');
    assert.match(b.audio.failure(), /reload/i);
});
