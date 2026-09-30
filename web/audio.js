// Main-thread bridge for a transferable, bounded AudioWorklet stream. The pool
// contains owned ArrayBuffers; WASM memory is copied, never transferred.
(() => {
    let context, node, starting = false, epoch = 0, playing = false, gain = 0.5;
    let error = '', stats = { depth: 0, maxDepth: 0, underruns: 0, overflows: 0, played: 0 };
    let pending = 0, bridgeDrops = 0;
    const pool = [];
    let timer;
    const message = data => node?.port.postMessage(data);
    globalThis.gbaAudio = {
        start() {
            if (starting) return;
            error = '';
            try {
                // resume() is called synchronously in the gesture, before module loading.
                context ??= new AudioContext({ latencyHint: 'interactive' });
                const resumed = context.resume();
                starting = true;
                (async () => {
                    await resumed;
                    if (!node) {
                        await context.audioWorklet.addModule(new URL('pcm-worklet.js', document.baseURI));
                        node = new AudioWorkletNode(context, 'gba-pcm', {
                            numberOfInputs: 0, numberOfOutputs: 1, outputChannelCount: [2],
                            processorOptions: { capacity: Math.ceil(context.sampleRate * 0.08),
                                target: Math.ceil(context.sampleRate * 0.06) }
                        });
                        node.onprocessorerror = () => { error = 'AudioWorklet processor failed; reload to retry'; };
                        node.port.onmessage = ({ data }) => {
                            if (data.kind === 'recycle') {
                                pending = Math.max(0, pending - data.frames);
                                if (pool.length < 8) pool.push(data.buffer);
                            } else if (data.kind === 'stats') {
                                if (data.epoch === epoch) stats = data;
                            }
                        };
                        node.connect(context.destination);
                        timer = setInterval(() => message({ kind: 'stats' }), 500);
                    }
                    message({ kind: 'clear', epoch });
                    message({ kind: 'playing', value: playing });
                    message({ kind: 'gain', value: gain });
                })().catch(e => { error = String(e); }).finally(() => { starting = false; });
            } catch (e) { error = String(e); starting = false; }
        },
        rate() { return node && context.state === 'running' && !error ? context.sampleRate : 0; },
        submit(samples) {
            if (!node || !playing || context.state !== 'running' || error) return;
            // Bound messages in flight as well as the worklet ring; delayed port
            // delivery cannot build an unbounded queue on a busy browser thread.
            if (pending + samples.length > Math.ceil(context.sampleRate * 0.08)) {
                bridgeDrops += samples.length;
                return;
            }
            const bytes = samples.length * 4;
            let index = pool.findIndex(buffer => buffer.byteLength >= bytes);
            const buffer = index < 0 ? new ArrayBuffer(Math.max(bytes, 16384 * 4)) : pool.splice(index, 1)[0];
            new Float32Array(buffer, 0, samples.length).set(samples);
            pending += samples.length;
            node.port.postMessage({ kind: 'pcm', epoch, frames: samples.length, buffer }, [buffer]);
        },
        clear() { epoch++; stats.depth = 0; message({ kind: 'clear', epoch }); },
        setPlaying(value) {
            if (playing === value) return;
            playing = value;
            this.clear();
            message({ kind: 'playing', value });
        },
        setGain(value) {
            if (gain === value) return;
            gain = value;
            message({ kind: 'gain', value });
        },
        status() {
            if (error) return `Audio unavailable: ${error}`;
            if (!node) return starting ? 'Starting audio…' : 'Click Enable audio to retry';
            return `${context.sampleRate} Hz (${context.state}) | queue ${(stats.depth * 1000 / context.sampleRate).toFixed(1)} ms (max ${(stats.maxDepth * 1000 / context.sampleRate).toFixed(1)}, cap 80) | underrun frames ${stats.underruns} | overflow frames ${stats.overflows + bridgeDrops} | output frames ${stats.played}`;
        }
    };
    // Visibility suspension clears the ring immediately rather than waiting for
    // a throttled app repaint. Re-entry primes from newly produced samples.
    document.addEventListener('visibilitychange', () => { if (document.hidden) globalThis.gbaAudio.setPlaying(false); });
    window.addEventListener('blur', () => globalThis.gbaAudio.setPlaying(false));
    window.addEventListener('pagehide', () => { globalThis.gbaAudio.setPlaying(false); clearInterval(timer); });
})();
