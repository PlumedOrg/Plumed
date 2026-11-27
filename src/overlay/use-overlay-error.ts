import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { i18n } from '@/i18n';

const DISMISS_MS = 4000;

export const useOverlayError = () => {
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const unlistenRecordingError = listen<string>('recording-error', () => {
            setError('Mic error');
        });
        const unlistenChunkError = listen<string>('transcription-chunk-error', () => {
            setError('Transcription partial');
        });
        // Plumed fork (written with Claude Code): dictation is refused while
        // the Réunion page transcribes a meeting.
        const unlistenMeetingBusy = listen('meeting-busy', () => {
            setError(i18n.t('Meeting mode is listening'));
        });
        return () => {
            unlistenMeetingBusy.then((u) => u());
            unlistenRecordingError.then((u) => u());
            unlistenChunkError.then((u) => u());
        };
    }, []);

    useEffect(() => {
        if (error == null) return;
        const timer = setTimeout(() => setError(null), DISMISS_MS);
        return () => clearTimeout(timer);
    }, [error]);

    return error;
};
