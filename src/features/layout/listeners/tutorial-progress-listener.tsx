import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { HistoryEntry } from '@/features/home/history/hooks/use-history-state';
import { markDictatedExamples } from '@/features/tutorial/tutorial-progress';

/**
 * Plumed fork (written with Claude Code): marks the tutorial examples found in
 * the recent dictations as used, at start and after each new dictation.
 */
export const TutorialProgressListener = () => {
    useEffect(() => {
        const check = () =>
            invoke<HistoryEntry[]>('get_recent_transcriptions')
                .then(markDictatedExamples)
                .catch((e: unknown) => console.error('Failed to check the tutorial examples:', e));
        void check();
        const unlisten = listen('history-updated', () => void check());
        return () => {
            unlisten.then((fn) => fn());
        };
    }, []);

    return null;
};
