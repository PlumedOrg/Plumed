import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';

/** A PhoneMic transcript that arrived too late is kept in the history, not pasted. */
export const RemoteMicLateListener = () => {
    const { t } = useTranslation();

    useEffect(() => {
        const unlisten = listen('remote-mic-late-transcript', () => {
            toast.warning(t('Late PhoneMic transcript: saved in the history, not pasted'), { autoClose: 8000 });
        });

        return () => {
            unlisten.then((fn) => fn());
        };
    }, [t]);

    return null;
};
