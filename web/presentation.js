'use strict';
// DOM activation belongs to the event callback, not the later egui paint. Rust
// publishes only visible button bounds/focus; this adapter owns browser promises.
(() => {
    let regions = {}, configured = false, fullscreenError = '', pending = false;
    let canvas, pressed, repaint;
    const changed = () => repaint?.();
    function toggleFullscreen() {
        if (pending) return;
        fullscreenError = '';
        try {
            const request = document.fullscreenElement === canvas
                ? document.exitFullscreen() : canvas.requestFullscreen();
            pending = true;
            Promise.resolve(request).catch(error => { fullscreenError = String(error); })
                .finally(() => { pending = false; changed(); });
        } catch (error) { fullscreenError = String(error); changed(); }
    }
    globalThis.gbaPresentation = {
        configure(gain, notify) { globalThis.gbaAudio.setGain(gain); repaint = notify; configured = true; },
        changed,
        beginFrame() { regions = {}; },
        // Coordinates are normalized to the egui viewport, so browser zoom and
        // device pixel ratio do not change the event hit-test contract.
        region(name, left, top, right, bottom, focused) {
            regions[name] = { left, top, right, bottom, focused };
        },
        fullscreen() { return !!canvas && document.fullscreenElement === canvas; },
        error() { return fullscreenError; },
    };
    function install() {
        canvas = document.getElementById('gba_canvas');
        if (!canvas) return;
        function activate(event) {
            if (!event.isTrusted || event.target !== canvas || document.hidden) return;
            if (event.type === 'keydown' && (event.repeat || event.ctrlKey || event.altKey
                || event.metaKey || event.key === 'Escape' || event.key === 'F11')) return;
            if (event.type === 'pointerup' && event.button !== 0) return;
            const bounds = canvas.getBoundingClientRect();
            const x = (event.clientX - bounds.left) / bounds.width;
            const y = (event.clientY - bounds.top) / bounds.height;
            const hit = name => {
                const r = regions[name];
                return r && (event.type === 'keydown'
                    ? r.focused && (event.key === 'Enter' || event.key === ' ')
                    : x >= r.left && x <= r.right && y >= r.top && y <= r.bottom);
            };
            // Retry is deliberately scoped to its visible control. Other gestures
            // unlock suspended audio but never silently retry a known failure.
            const click = name => hit(name) && (event.type === 'keydown'
                || (pressed?.id === event.pointerId && pressed[name]));
            if (configured) globalThis.gbaAudio.start(!!click('audio'));
            if (click('fullscreen')) toggleFullscreen();
            if (event.type === 'pointerup') pressed = undefined;
        }
        canvas.addEventListener('pointerdown', event => {
            if (!event.isTrusted || event.button !== 0) return;
            const b = canvas.getBoundingClientRect();
            const x = (event.clientX - b.left) / b.width;
            const y = (event.clientY - b.top) / b.height;
            pressed = { id: event.pointerId };
            for (const name of ['fullscreen', 'audio']) {
                const r = regions[name];
                pressed[name] = r && x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
            }
        });
        canvas.addEventListener('pointercancel', () => { pressed = undefined; });
        canvas.addEventListener('pointerup', activate);
        canvas.addEventListener('keydown', activate);
        document.addEventListener?.('fullscreenchange', changed);
    }
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', install, { once: true });
    } else { install(); }
})();
