// Packets sent to the desktop and not acknowledged yet (OP.ACK). The desktop
// ignores anything after a gap (a poll answer that died on its way, or a
// packet withheld by the relay), so whatever it has not acknowledged after
// RESEND_AFTER_MS is sent again, in order. Keep
// PhoneMic_server/protocol_details.md in sync with any change here.

export const RESEND_AFTER_MS = 4000;
// About 4 minutes of 16 kHz audio. Beyond it the desktop is not answering:
// everything is dropped, and the next START resynchronises it.
export const MAX_BYTES = 8 * 1024 * 1024;

export class Outbox {
    constructor() {
        /** [{seq, packet, sentAt}] in sequence order */
        this.items = [];
        this.bytes = 0;
    }

    add(seq, packet, now) {
        this.items.push({ seq, packet, sentAt: now });
        this.bytes += packet.length;
        if (this.bytes > MAX_BYTES) this.clear();
    }

    /** The desktop received everything up to `seq`. */
    ack(seq) {
        const keep = this.items.findIndex((it) => it.seq > seq);
        const gone = keep === -1 ? this.items : this.items.slice(0, keep);
        this.bytes -= gone.reduce((n, it) => n + it.packet.length, 0);
        this.items = keep === -1 ? [] : this.items.slice(keep);
    }

    /** -> packets to send again: all unacknowledged ones once the oldest is overdue. */
    due(now) {
        if (this.items.length === 0 || now - this.items[0].sentAt < RESEND_AFTER_MS) return [];
        for (const it of this.items) it.sentAt = now;
        return this.items.map((it) => it.packet);
    }

    clear() {
        this.items = [];
        this.bytes = 0;
    }
}
