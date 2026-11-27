// Plumed fork (written with Claude Code): the shapes of the Réunion commands
// and events (see src-tauri/src/meeting/state.rs and worker.rs).

export interface MeetingLine {
    /** Voice id: 0 is A, 1 is B... Sortformer hears up to four speakers at once, a long meeting can have more (gaps after merges). */
    speaker: number;
    /** Seconds since the start of the meeting. */
    start: number;
    end: number;
    text: string;
}

/** `meeting-lines` event: the lines added or grown (`meeting-lines-reset` sends them all after a voice merge). */
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

/**
 * When a recording started: the meeting audio from `audioSecs` on was said
 * from `wallMs` (Unix ms) on. `null` for a file or PhoneMic, whose audio has
 * no known clock time (PhoneMic only sends the recordings, with gaps between them).
 */
export interface ClockAnchor {
    audioSecs: number;
    wallMs: number | null;
}

export interface TimestampOptions {
    enabled: boolean;
    /** "HH:MM" clock time of the start, or "" for the real clock time where `anchors` know it, else the time since the start. */
    startClock: string;
    /** Seconds added to every time (negative allowed). */
    offsetSecs: number;
    /** One per recording, in order (see `useMeeting`). */
    anchors: ClockAnchor[];
}
