// Audio thread: preallocated mono ring, duplicated to output channels. The
// process callback performs no allocation, logging, messaging or machine work.
class GbaPcm extends AudioWorkletProcessor {
    constructor(options) {
        super();
        const config = options.processorOptions;
        this.ring = new Float32Array(config.capacity);
        this.target = config.target;
        this.read = 0; this.write = 0; this.depth = 0; this.epoch = 0;
        this.active = false; this.primed = false; this.gain = 0.5;
        this.maxDepth = 0; this.underruns = 0; this.overflows = 0; this.played = 0;
        this.port.onmessage = ({ data }) => {
            if (data.kind === 'clear') {
                this.epoch = data.epoch;
                this.read = 0; this.write = 0; this.depth = 0; this.primed = false;
            } else if (data.kind === 'playing') {
                this.active = data.value;
            } else if (data.kind === 'gain') {
                this.gain = data.value;
            } else if (data.kind === 'pcm') {
                if (data.epoch === this.epoch && this.active) {
                    const samples = new Float32Array(data.buffer, 0, data.frames);
                    for (let i = 0; i < samples.length; i++) {
                        if (this.depth === this.ring.length) { this.overflows++; continue; }
                        this.ring[this.write] = samples[i];
                        this.write = (this.write + 1) % this.ring.length;
                        this.depth++;
                    }
                    this.maxDepth = Math.max(this.maxDepth, this.depth);
                }
                // Copy completed; return only the owned buffer for reuse. This
                // message handler is outside the real-time process callback.
                this.port.postMessage({ kind: 'recycle', epoch: data.epoch,
                    frames: data.frames, buffer: data.buffer }, [data.buffer]);
            } else if (data.kind === 'stats') {
                this.port.postMessage({ kind: 'stats', epoch: this.epoch, depth: this.depth,
                    maxDepth: this.maxDepth, underruns: this.underruns,
                    overflows: this.overflows, played: this.played });
            }
        };
    }

    process(_inputs, outputs) {
        const channels = outputs[0];
        if (!channels.length) return true;
        if (this.active && !this.primed && this.depth >= this.target) this.primed = true;
        let missing = false;
        for (let frame = 0; frame < channels[0].length; frame++) {
            let sample = 0;
            if (this.active && this.primed) {
                if (this.depth) {
                    sample = this.ring[this.read] * this.gain;
                    this.read = (this.read + 1) % this.ring.length;
                    this.depth--; this.played++;
                } else {
                    this.underruns++; missing = true;
                }
            }
            for (let channel = 0; channel < channels.length; channel++) channels[channel][frame] = sample;
        }
        if (missing) this.primed = false;
        return true;
    }
}
registerProcessor('gba-pcm', GbaPcm);
