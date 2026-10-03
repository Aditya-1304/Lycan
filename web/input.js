'use strict';
(() => {
    let gameplay = false, revision = 0;
    let bindings = new Set();

    // Retain interruptions across throttled/missed animation frames. Rust observes
    // the revision and enters the existing session/audio suspension boundary once.
    function interrupt() {
        gameplay = false;
        revision = (revision + 1) >>> 0;
    }
    globalThis.gbaInput = {
        lifecycleRevision() { return revision; },
        setGameplay(value) { gameplay = value; },
        setBindings(keys) { bindings = new Set(keys); },
    };
    function install() {
        const canvas = document.getElementById('gba_canvas');
        if (!canvas) return;
        canvas.addEventListener('keydown', event => {
            if (!gameplay || document.hidden || document.activeElement !== canvas
                || event.target !== canvas || event.ctrlKey || event.altKey
                || event.metaKey || event.shiftKey) return;
            // Match the stable logical names serialized by the application.
            let key = event.key;
            if (key.startsWith('Arrow')) key = key.slice(5);
            else if (key === ' ') key = 'Space';
            else if (key.length === 1) key = key.toUpperCase();
            if (bindings.has(key)) event.preventDefault();
            // Do not stop propagation: eframe must still receive key transitions.
        });
    }
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', install, { once: true });
    } else {
        install();
    }
    document.addEventListener('visibilitychange', () => { if (document.hidden) interrupt(); });
    window.addEventListener('blur', interrupt);
    window.addEventListener('pagehide', interrupt);
})();
