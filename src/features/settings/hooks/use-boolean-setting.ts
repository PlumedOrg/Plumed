import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { toast } from 'react-toastify';
import { AppSettings } from '@/features/settings/settings.types';

export type BooleanSettingKey = {
    [K in keyof AppSettings]: AppSettings[K] extends boolean ? K : never;
}[keyof AppSettings];

/**
 * Loads one boolean setting and saves it through its Tauri command (called
 * with `{ ...args, enabled }`), rolling back on failure.
 */
export const useBooleanSetting = (
    key: BooleanSettingKey,
    command: string,
    errorMessage: string,
    args?: Record<string, unknown>
) => {
    const [value, setValue] = useState<boolean>(false);

    useEffect(() => {
        invoke<AppSettings>('get_all_settings').then((settings) => {
            setValue(settings[key]);
        });
    }, [key]);

    const save = async (enabled: boolean) => {
        try {
            setValue(enabled);
            await invoke(command, { ...args, enabled });
        } catch {
            toast.error(errorMessage);
            setValue(!enabled);
        }
    };

    return [value, save] as const;
};
