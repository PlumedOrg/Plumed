import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { toast } from 'react-toastify';
import { AppSettings } from '@/features/settings/settings.types';

type BooleanSettingKey = {
    [K in keyof AppSettings]: AppSettings[K] extends boolean ? K : never;
}[keyof AppSettings];

/** Loads one boolean setting and saves it through its Tauri command, rolling back on failure. */
export const useBooleanSetting = (key: BooleanSettingKey, command: string, errorMessage: string) => {
    const [value, setValue] = useState<boolean>(false);

    useEffect(() => {
        invoke<AppSettings>('get_all_settings').then((settings) => {
            setValue(settings[key]);
        });
    }, [key]);

    const save = async (enabled: boolean) => {
        try {
            setValue(enabled);
            await invoke(command, { enabled });
        } catch {
            toast.error(errorMessage);
            setValue(!enabled);
        }
    };

    return [value, save] as const;
};
