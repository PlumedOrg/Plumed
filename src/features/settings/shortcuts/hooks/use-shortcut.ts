import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'react-toastify';
import {
    findConflict,
    ExistingShortcut,
} from '../shortcuts-regular/shortcut-button/hooks/use-shortcut-interactions.helpers';

interface UseShortcutOptions {
    defaultShortcut: string;
    getCommand: string;
    setCommand: string;
    // Plumed: backend event after which the stored value changed (reloaded)
    changedEvent?: string;
}

export const useShortcut = ({ defaultShortcut, getCommand, setCommand, changedEvent }: UseShortcutOptions) => {
    const [shortcut, setShortcut] = useState(defaultShortcut);
    const { t } = useTranslation();

    useEffect(() => {
        const load = () =>
            invoke<string>(getCommand)
                .then((val) => val != null && setShortcut(val))
                .catch((err) => console.error(`Failed to load shortcut (${getCommand}):`, err));
        void load();
        if (changedEvent == null) return;
        const unlisten = listen(changedEvent, () => {
            void load();
            toast.info(t('The cancel shortcut was removed: the record shortcut contains its keys.'));
        });
        return () => {
            void unlisten.then((stop) => stop());
        };
    }, [getCommand, changedEvent, t]);

    const saveShortcut = async (value: string) => {
        if (value == null) return;
        try {
            const normalized = await invoke<string>(setCommand, { binding: value });
            setShortcut(normalized);
        } catch (err) {
            toast.error(
                err === 'cancel_inside_record'
                    ? t('The cancel shortcut cannot be made only of keys of the record shortcut.')
                    : t('Failed to save shortcut')
            );
        }
    };

    const resetShortcut = (existingShortcuts: ExistingShortcut[] = []) => {
        const conflict = findConflict(defaultShortcut, existingShortcuts);
        if (conflict != null) {
            toast.error(t('Cannot reset: default shortcut is already used by "{{name}}".', { name: conflict }));
            return;
        }
        setShortcut(defaultShortcut);
        void saveShortcut(defaultShortcut);
    };

    return {
        shortcut,
        setShortcut: saveShortcut,
        resetShortcut,
    };
};

export const SHORTCUT_CONFIGS = {
    lastTranscript: {
        defaultShortcut: 'ctrl+shift+space',
        getCommand: 'get_last_transcript_shortcut',
        setCommand: 'set_last_transcript_shortcut',
    },
    record: {
        defaultShortcut: 'ctrl+space',
        getCommand: 'get_record_shortcut',
        setCommand: 'set_record_shortcut',
    },
    cancel: {
        defaultShortcut: 'escape',
        getCommand: 'get_cancel_shortcut',
        setCommand: 'set_cancel_shortcut',
        changedEvent: 'cancel-shortcut-cleared',
    },
};
