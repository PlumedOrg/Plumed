import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ModelStatus } from '../types';

/** The backend's "the installed or active model changed" event. */
export const MODEL_STATUS_CHANGED_EVENT = 'model-status-changed';

// Plumed fork (written with Claude Code): the catalog with what is installed,
// refreshed whenever the backend says the model changed.
export const useModelStatus = () => {
    const [status, setStatus] = useState<ModelStatus | null>(null);

    const refresh = useCallback(() => {
        invoke<ModelStatus>('get_model_status')
            .then(setStatus)
            .catch((e) => console.error('Failed to read the model status:', e));
    }, []);

    useEffect(() => {
        refresh();
        const unlisten = listen(MODEL_STATUS_CHANGED_EVENT, refresh);
        return () => {
            void unlisten.then((stop) => stop());
        };
    }, [refresh]);

    return { status, setStatus, refresh };
};
