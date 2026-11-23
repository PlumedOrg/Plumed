import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { toast } from 'react-toastify';
import { AppSettings, AppUpdateChannel } from '@/features/settings/settings.types';
import { useTranslation } from '@/i18n';

// Plumed fork (written with Claude Code): the app update channel, shared by
// the sidebar update button and Settings > Online. The backend announces each
// change ("app-update-channel"), so every user of this hook stays in sync.
// `null` until the settings are loaded.
export const useAppUpdateChannel = () => {
    const { t } = useTranslation();
    const [channel, setChannel] = useState<AppUpdateChannel | null>(null);

    useEffect(() => {
        invoke<AppSettings>('get_all_settings')
            .then((settings) => setChannel(settings.app_update_channel))
            .catch((e) => console.error('Failed to load the update channel:', e));
        const unlisten = listen<AppUpdateChannel>('app-update-channel', (event) => setChannel(event.payload));
        return () => {
            void unlisten.then((fn) => fn());
        };
    }, []);

    const save = async (next: AppUpdateChannel) => {
        try {
            await invoke('set_app_update_channel', { channel: next });
        } catch {
            toast.error(t('Failed to save the app updates setting'));
        }
    };

    return [channel, save] as const;
};
