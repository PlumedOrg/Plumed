import { MeetingLine, TimestampOptions } from './meeting.types';

export const DEFAULT_SPEAKER_NAMES = ['A', 'B', 'C', 'D'];

export const AUDIO_EXTENSIONS = ['wav', 'mp3', 'm4a', 'mp4', 'aac', 'flac', 'ogg'];

const DAY_SECS = 24 * 3600;

const pad = (n: number) => String(n).padStart(2, '0');

/** 3725.4 -> "01:02:05" */
export const formatDuration = (secs: number): string => {
    const s = Math.max(0, Math.floor(secs));
    return `${pad(Math.floor(s / 3600))}:${pad(Math.floor((s % 3600) / 60))}:${pad(s % 60)}`;
};

/** The time of a line: since the start, or a clock time when `startClock` is set ("14:00" + 65 s -> "14:01:05"). */
export const lineTimestamp = (start: number, options: TimestampOptions): string => {
    const shifted = start + options.offsetSecs;
    const clock = /^(\d{1,2}):(\d{2})$/.exec(options.startClock.trim());
    if (clock == null) {
        return formatDuration(shifted);
    }
    const secs = Number(clock[1]) * 3600 + Number(clock[2]) * 60 + shifted;
    return formatDuration(((secs % DAY_SECS) + DAY_SECS) % DAY_SECS);
};

export const speakerName = (names: string[], speaker: number): string =>
    names[speaker]?.trim() || DEFAULT_SPEAKER_NAMES[speaker] || String(speaker + 1);

/** "- [00:01:05] Paul: bla" */
export const formatLine = (line: MeetingLine, names: string[], options: TimestampOptions): string => {
    const time = options.enabled ? `[${lineTimestamp(line.start, options)}] ` : '';
    return `- ${time}${speakerName(names, line.speaker)}: ${line.text}`;
};

export const formatTranscript = (lines: MeetingLine[], names: string[], options: TimestampOptions): string =>
    lines.map((line) => formatLine(line, names, options)).join('\n');
