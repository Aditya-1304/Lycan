'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const test = require('node:test');

// The actual DOM adapter must distinguish a click from releasing a drag over
// Fullscreen. Rust widget tests cannot exercise browser activation or promises.
test('fullscreen uses a real click, observes rejection/external exit, and supports focused keyboard activation', async () => {
    const listeners = {};
    let requests = 0, reject = true, audioGestures = 0;
    const document = { readyState: 'complete', hidden: false, fullscreenElement: null,
        getElementById() { return canvas; },
        exitFullscreen() { this.fullscreenElement = null; return Promise.resolve(); } };
    const canvas = {
        addEventListener(name, listener) { listeners[name] = listener; },
        getBoundingClientRect() { return { left: 0, top: 0, width: 800, height: 600 }; },
        requestFullscreen() {
            requests++;
            if (reject) return Promise.reject(new Error('denied'));
            document.fullscreenElement = this;
            return Promise.resolve();
        },
    };
    const host = vm.createContext({ document, gbaAudio: {
        setGain() {}, start() { audioGestures++; },
    } });
    vm.runInContext(fs.readFileSync(`${__dirname}/presentation.js`, 'utf8'), host);
    const api = host.gbaPresentation;
    api.configure(0);
    api.region('fullscreen', 0.5, 0, 1, 0.2, false);
    function pointer(type, x) {
        listeners[type]?.({ type, isTrusted: true, target: canvas, button: 0,
            pointerId: 1, clientX: x, clientY: 30 });
    }
    pointer('pointerdown', 10);
    pointer('pointerup', 700);
    assert.equal(requests, 0);
    pointer('pointerdown', 700);
    pointer('pointerup', 700);
    assert.equal(requests, 1);
    assert.equal(api.fullscreen(), false);
    await new Promise(resolve => setImmediate(resolve));
    assert.match(api.error(), /denied/);
    reject = false;
    api.region('fullscreen', 0.5, 0, 1, 0.2, true);
    listeners.keydown({ type: 'keydown', isTrusted: true, target: canvas, key: 'Enter' });
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(api.fullscreen(), true);
    document.fullscreenElement = null;
    assert.equal(api.fullscreen(), false);
    const beforeF11 = audioGestures;
    listeners.keydown({ type: 'keydown', isTrusted: true, target: canvas, key: 'F11' });
    assert.equal(requests, 2);
    assert.equal(audioGestures, beforeF11);
    api.beginFrame();
    listeners.keydown({ type: 'keydown', isTrusted: true, target: canvas, key: 'Enter' });
    assert.equal(requests, 2);
});
