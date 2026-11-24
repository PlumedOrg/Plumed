export type RemoteMicStatus = 'stopped' | 'starting' | 'waiting' | 'verify' | 'connected';

/** Mirrors `remote_mic::session::Snapshot` (Rust). */
export interface RemoteMicSnapshot {
    status: RemoteMicStatus;
    qr: string | null;
    link: string | null;
    fingerprint: string | null;
    error: string | null;
    /** Values interpolated into `error` (a translation key). */
    error_params: Record<string, string> | null;
    /** The relay is on a newer protocol: offer to update Plumed, even with the automatic check disabled. */
    update_app: boolean;
}
