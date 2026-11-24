// Microphone capture, audio file decoding and PCM conversion.

export const TARGET_RATE = 16000;

/**
 * Streaming box-filter downsampler to 16 kHz (each output sample is the mean
 * of the ~3 input samples it covers at 48 kHz). Rates at or below 16 kHz pass
 * through unchanged, the desktop accepts any rate announced in START.
 */
export class Downsampler {
    constructor(inRate) {
        this.ratio = inRate / TARGET_RATE;
        this.rate = this.ratio > 1 ? TARGET_RATE : inRate;
        this.acc = 0;
        this.n = 0;
        this.phase = 0;
    }

    push(input) {
        if (this.ratio <= 1) return input;
        const out = new Float32Array(Math.ceil(input.length / this.ratio) + 1);
        let k = 0;
        for (const x of input) {
            this.acc += x;
            this.n += 1;
            this.phase += 1;
            if (this.phase >= this.ratio) {
                out[k++] = this.acc / this.n;
                this.acc = 0;
                this.n = 0;
                this.phase -= this.ratio;
            }
        }
        return out.subarray(0, k);
    }
}

/** Float32 [-1, 1] -> PCM s16le bytes. */
export function toPcm16(samples) {
    const out = new Uint8Array(samples.length * 2);
    const view = new DataView(out.buffer);
    for (let i = 0; i < samples.length; i++) {
        const s = Math.max(-1, Math.min(1, samples[i]));
        view.setInt16(i * 2, s < 0 ? s * 0x8000 : s * 0x7fff, true);
    }
    return out;
}

export class Capture {
    /** Opens the microphone, onChunk(Float32Array at capture.rate). */
    static async open(onChunk) {
        const stream = await navigator.mediaDevices.getUserMedia({
            audio: { channelCount: 1, echoCancellation: true, noiseSuppression: true, autoGainControl: true },
        });
        const ctx = new AudioContext();
        try {
            await ctx.audioWorklet.addModule('worklet.js');
            const source = ctx.createMediaStreamSource(stream);
            const node = new AudioWorkletNode(ctx, 'plumed-capture');
            const down = new Downsampler(ctx.sampleRate);
            node.port.onmessage = (e) => onChunk(down.push(e.data));
            source.connect(node);
            // Some browsers only run nodes connected to the output (it plays silence)
            node.connect(ctx.destination);
            if (ctx.state === 'suspended') await ctx.resume();
            return new Capture(stream, ctx, node, down.rate);
        } catch (e) {
            stream.getTracks().forEach((t) => t.stop());
            await ctx.close();
            throw e;
        }
    }

    constructor(stream, ctx, node, rate) {
        this.stream = stream;
        this.ctx = ctx;
        this.node = node;
        this.rate = rate;
    }

    close() {
        this.node.port.onmessage = null;
        this.stream.getTracks().forEach((t) => t.stop());
        this.ctx.close();
    }
}

/** Checked before reading: a phone would run out of memory decoding a huge
 * file just to learn it is too long (20 min of 48 kHz stereo WAV is 220 MiB). */
export const MAX_FILE_BYTES = 256 * 1024 * 1024;

export class FileTooLarge extends RangeError {}

/** Any audio file the browser can decode -> mono Float32 at 16 kHz. Throws
 * `FileTooLarge` past `MAX_FILE_BYTES`, `RangeError` past `maxSeconds`. */
export async function decodeFile(file, maxSeconds) {
    if (file.size > MAX_FILE_BYTES) throw new FileTooLarge('audio file too large');
    // decodeAudioData resamples to the context rate
    const ctx = new OfflineAudioContext(1, 1, TARGET_RATE);
    const decoded = await ctx.decodeAudioData(await file.arrayBuffer());
    if (decoded.duration > maxSeconds) throw new RangeError('audio file too long');
    const mono = new Float32Array(decoded.length);
    for (let c = 0; c < decoded.numberOfChannels; c++) {
        const data = decoded.getChannelData(c);
        for (let i = 0; i < data.length; i++) mono[i] += data[i] / decoded.numberOfChannels;
    }
    return mono;
}
