import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { ListFile } from '../corrections.types';

const FILE_FORMATS: Record<ListFile, { extension: string; defaultName: string }> = {
    rules: { extension: 'jsonl', defaultName: 'plumed-model-corrections.jsonl' },
    phrases: { extension: 'txt', defaultName: 'plumed-medical-vocabulary.txt' },
    confusion_groups: { extension: 'jsonl', defaultName: 'plumed-sound-alike-words.jsonl' },
};

/** Export and import of a medical list in its native format, through the file dialogs. */
export const useListFile = (list: ListFile, onImported: () => void) => {
    const { t } = useTranslation();
    const { extension, defaultName } = FILE_FORMATS[list];
    const filters = [{ name: extension.toUpperCase(), extensions: [extension] }];

    const exportList = async () => {
        try {
            const filePath = await save({ title: t('Export the list'), filters, defaultPath: defaultName });
            if (filePath == null) {
                return;
            }
            await invoke('export_medical_list', { list, filePath });
            toast.success(t('List exported'), { autoClose: 2000 });
        } catch (error) {
            toast.error(`${t('Failed to export the list')}: ${String(error)}`);
        }
    };

    const importList = async () => {
        try {
            const filePath = await open({ title: t('Import a list'), multiple: false, directory: false, filters });
            if (filePath == null) {
                return;
            }
            await invoke('import_medical_list', { list, filePath });
            onImported();
            toast.success(t('List imported'), { autoClose: 2000 });
        } catch (error) {
            toast.error(`${t('Nothing was imported')}: ${String(error)}`, { autoClose: false });
        }
    };

    return { exportList, importList };
};
