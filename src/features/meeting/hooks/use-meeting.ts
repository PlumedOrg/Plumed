import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { AUDIO_EXTENSIONS } from '../meeting.helpers';
import { IndexedLine, MeetingLine, MeetingSnapshot, MeetingSource } from '../meeting.types';

const FAILED = Symbol('failed');

/** A file being transcribed: the progress bar goes from `base` to `base + duration`. */
interface FileProgress {
    base: number;
    duration: number | null;
}

// Plumed fork (written with Claude Code): follows the Rust meeting and exposes
// its commands. Lines arrive about 30 s behind the audio (diarization delay).
export const useMeeting = () => {
    const { t } = useTranslation();
    const [lines, setLines] = useState<MeetingLine[]>([]);
    const [processedSecs, setProcessedSecs] = useState(0);
    const [running, setRunning] = useState(false);
    const [starting, setStarting] = useState(false);
    const [stopping, setStopping] = useState(false);
    const [file, setFile] = useState<FileProgress | null>(null);

    useEffect(() => {
        invoke<MeetingSnapshot>('meeting_snapshot').then((snapshot) => {
            setLines(snapshot.lines);
            setProcessedSecs(snapshot.processed_secs);
            setRunning(snapshot.running);
        });
        const subscriptions = [
            listen<IndexedLine[]>('meeting-lines', (event) =>
                setLines((previous) => {
                    const next = [...previous];
                    for (const { index, ...line } of event.payload) {
                        next[index] = line;
                    }
                    return next;
                })
            ),
            // voices were merged: past lines changed and joined
            listen<MeetingLine[]>('meeting-lines-reset', (event) => setLines(event.payload)),
            listen<number>('meeting-progress', (event) => setProcessedSecs(event.payload)),
            listen('meeting-idle', () => {
                setRunning(false);
                setStopping(false);
            }),
        ];
        return () => {
            subscriptions.forEach((subscription) => void subscription.then((stop) => stop()));
        };
    }, []);

    const run = async (command: string, args?: Record<string, unknown>): Promise<unknown> => {
        setStarting(true);
        try {
            const result = await invoke(command, args);
            setRunning(true);
            return result;
        } catch (e) {
            toast.error(`${t('Could not start the meeting')}: ${String(e)}`);
            return FAILED;
        } finally {
            setStarting(false);
        }
    };

    const start = async (source: MeetingSource) => {
        if (source !== 'file') {
            setFile(null);
            await run(source === 'mic' ? 'meeting_start_mic' : 'meeting_start_phone');
            return;
        }
        const path = await open({ multiple: false, filters: [{ name: t('Audio'), extensions: AUDIO_EXTENSIONS }] });
        if (typeof path !== 'string') {
            return;
        }
        const base = processedSecs;
        const duration = await run('meeting_start_file', { path });
        if (duration !== FAILED) {
            setFile({ base, duration: duration as number | null });
        }
    };

    const stop = async () => {
        setStopping(true);
        await invoke('meeting_stop');
    };

    const reset = async () => {
        await invoke('meeting_reset');
        setLines([]);
        setProcessedSecs(0);
        setRunning(false);
        setStopping(false);
        setFile(null);
    };

    return { lines, processedSecs, running, starting, stopping, file, start, stop, reset };
};

export type Meeting = ReturnType<typeof useMeeting>;
