// Audio thread: forwards the microphone in chunks of 4096 samples (85 ms at
// 48 kHz) so the main thread is not woken 375 times per second.

const CHUNK = 4096;

class PlumedCapture extends AudioWorkletProcessor {
    constructor() {
        super();
        this.buf = new Float32Array(CHUNK);
        this.len = 0;
    }

    process(inputs) {
        const channel = inputs[0]?.[0];
        if (channel === undefined) return true;
        let i = 0;
        while (i < channel.length) {
            const n = Math.min(channel.length - i, CHUNK - this.len);
            this.buf.set(channel.subarray(i, i + n), this.len);
            this.len += n;
            i += n;
            if (this.len === CHUNK) {
                this.port.postMessage(this.buf, [this.buf.buffer]);
                this.buf = new Float32Array(CHUNK);
                this.len = 0;
            }
        }
        return true;
    }
}

registerProcessor('plumed-capture', PlumedCapture);
