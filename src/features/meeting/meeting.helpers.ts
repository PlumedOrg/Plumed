import { ClockAnchor, MeetingLine, TimestampOptions } from './meeting.types';

export const AUDIO_EXTENSIONS = ['wav', 'mp3', 'm4a', 'mp4', 'aac', 'flac', 'ogg'];

const DAY_SECS = 24 * 3600;

const pad = (n: number) => String(n).padStart(2, '0');

/** 3725.4 -> "01:02:05" */
export const formatDuration = (secs: number): string => {
    const s = Math.max(0, Math.floor(secs));
    return `${pad(Math.floor(s / 3600))}:${pad(Math.floor((s % 3600) / 60))}:${pad(s % 60)}`;
};

/** The anchor of the recording that holds `audioSecs`: the last one starting at or before it. */
const anchorAt = (anchors: ClockAnchor[], audioSecs: number): ClockAnchor | undefined => {
    const before = anchors.filter((anchor) => anchor.audioSecs <= audioSecs);
    return before[before.length - 1];
};

/**
 * The time of a line, in this order:
 * - a clock time from `startClock` when set ("14:00" + 65 s -> "14:01:05");
 * - the real clock time when its recording has one (microphone started at
 *   14:03:10, line 65 s into it -> "14:04:15");
 * - else the time since the start ("00:01:05").
 * `offsetSecs` shifts all three.
 */
export const lineTimestamp = (start: number, options: TimestampOptions): string => {
    const shifted = start + options.offsetSecs;
    const clock = /^(\d{1,2}):(\d{2})$/.exec(options.startClock.trim());
    if (clock != null) {
        const secs = Number(clock[1]) * 3600 + Number(clock[2]) * 60 + shifted;
        return formatDuration(((secs % DAY_SECS) + DAY_SECS) % DAY_SECS);
    }
    const anchor = anchorAt(options.anchors, start);
    if (anchor?.wallMs == null) {
        return formatDuration(shifted);
    }
    const said = new Date(anchor.wallMs + (shifted - anchor.audioSecs) * 1000);
    return `${pad(said.getHours())}:${pad(said.getMinutes())}:${pad(said.getSeconds())}`;
};

/** 0 -> "A", 25 -> "Z", 26 -> "27" (voice ids grow when speakers come and go). */
export const speakerLetter = (speaker: number): string =>
    speaker < 26 ? String.fromCharCode(65 + speaker) : String(speaker + 1);

export const speakerName = (names: string[], speaker: number): string =>
    names[speaker]?.trim() || speakerLetter(speaker);

/** "- [00:01:05] Paul: bla" */
export const formatLine = (line: MeetingLine, names: string[], options: TimestampOptions): string => {
    const time = options.enabled ? `[${lineTimestamp(line.start, options)}] ` : '';
    return `- ${time}${speakerName(names, line.speaker)}: ${line.text}`;
};

export const formatTranscript = (lines: MeetingLine[], names: string[], options: TimestampOptions): string =>
    lines.map((line) => formatLine(line, names, options)).join('\n');
