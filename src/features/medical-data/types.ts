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

/** Rust `DataStatus`. */
export interface MedicalDataStatus {
    available: boolean;
    bundled_version: string;
    downloaded_version: string | null;
    check_enabled: boolean;
}

export interface MedicalDataProgress {
    downloaded: number;
    total: number;
}
