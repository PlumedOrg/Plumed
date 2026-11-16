export type RemoteMicStatus = 'stopped' | 'starting' | 'waiting' | 'verify' | 'connected';

/** Mirrors `remote_mic::session::Snapshot` (Rust). */
export interface RemoteMicSnapshot {
    status: RemoteMicStatus;
    qr: string | null;
    link: string | null;
    fingerprint: string | null;
    error: string | null;
}
