import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ModelDownloadProgress, ModelStatus } from '../types';

// Plumed fork (written with Claude Code): starts, follows and cancels a model
// download. A cancelled or failed download keeps its files: starting it again
// resumes where it stopped. `download` defaults to the speech models; the
// Réunion page passes the diarization model's command.
const downloadModel = (variant: string) => invoke<ModelStatus>('download_model', { variant });

export const useModelDownload = <S = ModelStatus>(
    onStatus: (status: S) => void,
    download: (variant: string) => Promise<S> = downloadModel as (variant: string) => Promise<S>
) => {
    const [progress, setProgress] = useState<ModelDownloadProgress | null>(null);
    const [error, setError] = useState<unknown>(null);
    const [running, setRunning] = useState(false);

    useEffect(() => {
        const unlisten = listen<ModelDownloadProgress>('model-download-progress', (event) =>
            setProgress(event.payload)
        );
        return () => {
            void unlisten.then((stop) => stop());
        };
    }, []);

    const start = async (variant: string): Promise<boolean> => {
        setRunning(true);
        setError(null);
        setProgress(null);
        try {
            onStatus(await download(variant));
            return true;
        } catch (e) {
            setError(e);
            return false;
        } finally {
            setRunning(false);
        }
    };

    const cancel = () => {
        invoke('cancel_model_download').catch((e) => console.error('Failed to cancel the download:', e));
    };

    return { progress, error, running, start, cancel };
};

export type ModelDownload = ReturnType<typeof useModelDownload>;
