/** A list exported and imported in its native format (Rust `medical::ListFile`). */
export type ListFile = 'rules' | 'phrases' | 'confusion_groups' | 'grammar';

/** A heard form and the words it may stand for (Rust `ConfusionGroup`). */
export interface ConfusionGroup {
    heard: string;
    candidates: string[];
}

/** The user's changes to a bundled list (Rust `ListOverlay`). */
export interface ListOverlay<T> {
    /** Keys of the bundled entries turned off. */
    disabled: string[];
    added: T[];
}

/** 'orphaned': a disabled entry the medical data in use no longer has. */
export type EntryOrigin = 'bundled' | 'added' | 'orphaned';

export interface EntryView<T> {
    key: string;
    entry: T;
    origin: EntryOrigin;
    enabled: boolean;
}

export interface ListPage<T> {
    total: number;
    entries: EntryView<T>[];
}
