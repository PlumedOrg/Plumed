// Plumed fork (written with Claude Code): the shapes of the Réunion commands
// and events (see src-tauri/src/meeting/state.rs and worker.rs).

export interface MeetingLine {
    /** 0 to 3: Sortformer tracks up to four speakers. */
    speaker: number;
    /** Seconds since the start of the meeting. */
    start: number;
    end: number;
    text: string;
}

/** `meeting-lines` event: the lines added or grown. */
export interface IndexedLine extends MeetingLine {
    index: number;
}

export interface MeetingSnapshot {
    lines: MeetingLine[];
    processed_secs: number;
    running: boolean;
}

/** `meeting_backup_find`: a backup left by a previous run. */
export interface MeetingBackup {
    lines: number;
    processed_secs: number;
    has_audio: boolean;
    has_text: boolean;
    /** Unix seconds, when the progress was saved. */
    updated_at: number | null;
}

export type MeetingSource = 'mic' | 'phone' | 'file';

export interface TimestampOptions {
    enabled: boolean;
    /** "HH:MM" clock time of the start, or "" for times since the start. */
    startClock: string;
    /** Seconds added to every time (negative allowed). */
    offsetSecs: number;
}
