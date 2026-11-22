// Plumed fork (written with Claude Code): signed medical data updates, see
// `src-tauri/src/medical/data_update.rs`.

/** A newer signed data set found online (Rust `UpdateOffer`). */
export interface MedicalDataOffer {
    version: string;
    notes: string;
    file_count: number;
    total_size: number;
}

/** The data in use after a download or a reset (Rust `DataChange`). */
export interface MedicalDataChange {
    version: string;
    orphaned_edits: number;
}

/** Rust `CheckReport`: the last check of this launch. */
export type MedicalDataCheckReport = { checked_at: number } & (
    | { outcome: 'up_to_date' }
    | { outcome: 'update_found'; version: string }
    | { outcome: 'failed'; error: string }
);

/** Rust `DataStatus`. */
export interface MedicalDataStatus {
    available: boolean;
    bundled_version: string;
    downloaded_version: string | null;
    /** The startup check runs (the user agreed to it). */
    check_enabled: boolean;
    last_check: MedicalDataCheckReport | null;
}

/** Rust `SetError`: why downloaded files were refused. */
export type MedicalDataSetError =
    | { kind: 'missing_file'; name: string }
    | { kind: 'bad_signature'; detail: string }
    | { kind: 'invalid_manifest'; detail: string }
    | { kind: 'wrong_model'; model: string; expected: string }
    | { kind: 'app_too_old'; min_app_version: string }
    | { kind: 'not_newer'; version: string; active: string }
    | { kind: 'file_mismatch'; name: string }
    | { kind: 'other'; detail: string };

export interface MedicalDataProgress {
    downloaded: number;
    total: number;
}
