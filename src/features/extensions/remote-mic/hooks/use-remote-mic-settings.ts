import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { toast } from 'react-toastify';
import { AppSettings } from '@/features/settings/settings.types';
import { useBooleanSetting } from '@/features/settings/hooks/use-boolean-setting';
import { useTranslation } from '@/i18n';

export const useRemoteMicSettings = () => {
    const { t } = useTranslation();
    const [enabled, setEnabled] = useBooleanSetting(
        'remote_mic_enabled',
        'set_remote_mic_enabled',
        t('Failed to save the remote microphone setting')
    );
    const [url, setUrl] = useState('');
    const [savedUrl, setSavedUrl] = useState('');

    useEffect(() => {
        invoke<AppSettings>('get_all_settings').then((settings) => {
            setUrl(settings.remote_mic_url);
            setSavedUrl(settings.remote_mic_url);
        });
    }, []);

    const saveUrl = async () => {
        if (url === savedUrl) {
            return;
        }
        try {
            const origin = await invoke<string>('set_remote_mic_url', { url });
            setUrl(origin);
            setSavedUrl(origin);
        } catch (e) {
            toast.error(t(String(e)));
            setUrl(savedUrl);
        }
    };

    /** The relay dialog already saved `origin`: mirror it without saving again. */
    const applySavedUrl = (origin: string) => {
        setUrl(origin);
        setSavedUrl(origin);
    };

    return { enabled, setEnabled, url, setUrl, saveUrl, applySavedUrl };
};
