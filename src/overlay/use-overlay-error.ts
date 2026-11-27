import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { i18n } from '@/i18n';

const DISMISS_MS = 4000;

// Plumed fork (written with Claude Code): 'notice' is shown in purple, for
// outcomes that are not a failure of the app (nothing was heard).
export interface OverlayError {
    message: string;
    kind: 'error' | 'notice';
}

const failure = (message: string): OverlayError => ({ message, kind: 'error' });

export const useOverlayError = () => {
    const [error, setError] = useState<OverlayError | null>(null);

    useEffect(() => {
        const unlistenRecordingError = listen<string>('recording-error', () => {
            setError(failure('Mic error'));
        });
        const unlistenChunkError = listen<string>('transcription-chunk-error', () => {
            setError(failure('Transcription partial'));
        });
        // Plumed fork (written with Claude Code): dictation is refused while
        // the Réunion page transcribes a meeting.
        const unlistenMeetingBusy = listen('meeting-busy', () => {
            setError(failure(i18n.t('Meeting mode is listening')));
        });
        const unlistenNoSpeech = listen('no-speech-heard', () => {
            setError({ message: i18n.t('No sound was heard'), kind: 'notice' });
        });
        // a new dictation replaces whatever message is still showing
        const unlistenSessionStart = listen('overlay-session-start', () => setError(null));
        return () => {
            unlistenNoSpeech.then((u) => u());
            unlistenSessionStart.then((u) => u());
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
