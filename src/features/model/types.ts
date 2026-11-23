// Plumed fork (written with Claude Code): the shapes of the model commands
// and events (see src-tauri/src/model/manager.rs and download.rs).

export type ModelSource = 'hugging_face' | 'relay';

export interface ModelVariantStatus {
    id: string;
    family: string;
    name: string;
    /** A few English words, translated with `t`. */
    summary: string;
    /** Bytes. */
    size: number;
    /** False while the variant cannot run yet ("coming soon"). */
    available: boolean;
    /** Where the installed copy came from, null when it is not installed. */
    installed: ModelSource | null;
    /** Bytes already on disk: a download to resume. */
    downloaded: number;
}

export interface ModelStatus {
    active: string | null;
    /** A development build uses the model of resources/: nothing to download. */
    dev: boolean;
    downloading: string | null;
    variants: ModelVariantStatus[];
}

export interface ModelDownloadProgress {
    variant: string;
    downloaded: number;
    total: number;
    /** Bytes per second, smoothed. */
    speed: number;
    source: ModelSource;
}

export type ModelDownloadError =
    | { kind: 'network'; detail: string }
    | { kind: 'disk_space'; needed: number; available: number }
    | { kind: 'checksum'; file: string }
    | { kind: 'cancelled' }
    | { kind: 'busy' }
    | { kind: 'unavailable' }
    | { kind: 'io'; detail: string };
