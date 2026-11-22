import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { medicalDataSetErrorMessage } from '../medical-data.helpers';
import { MedicalDataChange } from '../types';
import { useMedicalDataChangeToast } from './use-medical-data-change-toast';

/**
 * "Install a downloaded update": the user picks the release assets downloaded with a browser
 * (several files at once works on every OS, unlike mixing files and folders). The backend runs
 * the same checks as an online update. Resolves once done, whatever the outcome.
 */
export const useInstallDownloadedFiles = (onInstalled: () => void) => {
    const { t } = useTranslation();
    const showChange = useMedicalDataChangeToast();

    return async () => {
        const paths = await open({
            title: t('Choose the downloaded files (manifest.json, manifest.json.minisig and the data files)'),
            multiple: true,
            directory: false,
        });
        if (paths == null || paths.length === 0) {
            return;
        }
        try {
            showChange(await invoke<MedicalDataChange>('install_medical_data_files', { paths }));
            onInstalled();
        } catch (e) {
            toast.error(`${t('Nothing was installed')}: ${medicalDataSetErrorMessage(e, t)}`, { autoClose: false });
        }
    };
};
