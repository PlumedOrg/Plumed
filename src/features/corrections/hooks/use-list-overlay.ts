import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { ListOverlay } from '../corrections.types';

const EMPTY_OVERLAY = { disabled: [], added: [] };

/**
 * The user's changes to a bundled list, loaded with `getCommand` and saved
 * whole with `setCommand` (the backend refuses them if one entry is invalid).
 */
export const useListOverlay = <T>(getCommand: string, setCommand: string) => {
    const { t } = useTranslation();
    const [overlay, setOverlay] = useState<ListOverlay<T>>(EMPTY_OVERLAY);

    const reload = useCallback(() => {
        invoke<ListOverlay<T>>(getCommand)
            .then(setOverlay)
            .catch((error: unknown) => console.error(`${getCommand} failed:`, error));
    }, [getCommand]);

    useEffect(reload, [reload]);

    const saveOverlay = useCallback(
        async (newOverlay: ListOverlay<T>): Promise<boolean> => {
            try {
                await invoke(setCommand, { overlay: newOverlay });
                setOverlay(newOverlay);
                return true;
            } catch (error) {
                toast.error(`${t('Invalid entry')}: ${String(error)}`);
                return false;
            }
        },
        [setCommand, t]
    );

    const changeCount = overlay.disabled.length + overlay.added.length;

    return { overlay, saveOverlay, reload, changeCount };
};
