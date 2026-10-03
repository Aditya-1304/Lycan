'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const vm = require('node:vm');

// Execute the production page guard with only EventTarget/DOM facts replaced.
// This catches browser defaults and between-frame transitions Rust tests cannot see.
function host() {
    class Target {
        constructor() { this.listeners = new Map(); }
        addEventListener(name, callback) {
            if (!this.listeners.has(name)) this.listeners.set(name, []);
            this.listeners.get(name).push(callback);
        }
        emit(name, event = {}) {
            for (const callback of this.listeners.get(name) || []) callback(event);
        }
    }
    const canvas = new Target();
    const document = new Target();
    Object.assign(document, {
        hidden: false, readyState: 'complete', activeElement: canvas,
        getElementById: id => id === 'gba_canvas' ? canvas : null,
    });
    const window = new Target();
    const context = vm.createContext({ document, window });
    vm.runInContext(fs.readFileSync(path.join(__dirname, 'input.js'), 'utf8'), context);
    function key(value, options = {}) {
        const event = { key: value, target: canvas, prevented: false,
            preventDefault() { this.prevented = true; }, ...options };
        canvas.emit('keydown', event);
        return event.prevented;
    }
    return { document, window, canvas, guard: context.gbaInput, key };
}

// A remapped Home/PageDown/Tab must not navigate/scroll the page, while settings,
// unrelated DOM targets and browser shortcuts must retain their normal behavior.
test('browser defaults are cancelled only for focused canvas gameplay bindings', () => {
    const { guard, document, key } = host();
    guard.setBindings(['Z', 'X', 'A', 'S', 'Enter', 'Backspace', 'Up', 'Home', 'Tab', 'PageDown']);
    guard.setGameplay(true);
    for (const value of ['z', 'Z', 'ArrowUp', 'Home', 'Tab', 'PageDown']) assert.equal(key(value), true);
    assert.equal(key('F11'), false);
    assert.equal(key('z', { ctrlKey: true }), false);
    assert.equal(key('Home', { target: {} }), false);
    document.activeElement = {};
    assert.equal(key('Home'), false);
    guard.setGameplay(false);
    assert.equal(key('Tab'), false);
});

// Hidden/blurred and returned tabs may have no intervening WASM callback. The
// revision must retain that interruption and disable the guard immediately.
test('lifecycle revision retains interruptions even when the next frame is visible', () => {
    const { guard, document, window, key } = host();
    guard.setBindings(['Home']);
    guard.setGameplay(true);
    const revision = guard.lifecycleRevision();
    document.hidden = true;
    document.emit('visibilitychange');
    document.hidden = false;
    document.emit('visibilitychange');
    assert.notEqual(guard.lifecycleRevision(), revision);
    assert.equal(key('Home'), false);
    guard.setGameplay(true);
    const afterHide = guard.lifecycleRevision();
    window.emit('blur');
    window.emit('focus');
    assert.notEqual(guard.lifecycleRevision(), afterHide);
    assert.equal(key('Home'), false);
    guard.setGameplay(true);
    window.emit('pagehide');
    assert.equal(key('Home'), false);
});
