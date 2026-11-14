import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { AppSettings } from '@/features/settings/settings.types';

export const useBlockCliImportState = () => {
    const [blockCliImport, setBlockCliImport] = useState<boolean>(false);
    const { t } = useTranslation();

    useEffect(() => {
        invoke<AppSettings>('get_all_settings').then((settings) => {
            setBlockCliImport(settings.block_cli_import);
        });
    }, []);

    const handleSetBlockCliImport = async (enabled: boolean) => {
        try {
            setBlockCliImport(enabled);
            await invoke('set_block_cli_import', { enabled });
        } catch {
            toast.error(t('Failed to save the command-line import setting'));
            setBlockCliImport(!enabled);
        }
    };

    return {
        blockCliImport,
        setBlockCliImport: handleSetBlockCliImport,
    };
};
