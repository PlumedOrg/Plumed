import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { RemoteMicSnapshot } from '../remote-mic.types';

const STOPPED: RemoteMicSnapshot = { status: 'stopped', qr: null, link: null, fingerprint: null, error: null, error_params: null };

/** Follows the Rust remote-mic session and exposes its commands. */
export const useRemoteMic = () => {
    const { t } = useTranslation();
    const [snapshot, setSnapshot] = useState<RemoteMicSnapshot>(STOPPED);

    useEffect(() => {
        invoke<RemoteMicSnapshot>('get_remote_mic_state').then(setSnapshot);
        const unlisten = listen<RemoteMicSnapshot>('remote-mic-state', (event) => setSnapshot(event.payload));
        return () => {
            unlisten.then((fn) => fn());
        };
    }, []);

    const start = async () => {
        try {
            await invoke('remote_mic_start');
        } catch (e) {
            toast.error(t(String(e)));
        }
    };

    return {
        snapshot,
        start,
        stop: () => invoke('remote_mic_stop'),
        confirm: (fingerprint: string, accepted: boolean) => invoke('remote_mic_confirm', { fingerprint, accepted }),
    };
};
