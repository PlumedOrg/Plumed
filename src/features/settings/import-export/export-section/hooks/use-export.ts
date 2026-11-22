import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { getVersion } from '@tauri-apps/api/app';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { CURRENT_MURMURE_FORMAT_VERSION, SUB_ITEM_KEY } from '../../import-export.constants';
import {
    CategoryKey,
    CategorySelection,
    MurmureExportData,
    ExportedCategories,
    AppSettings,
} from '../../import-export.types';
import { extractSystemSettings, extractShortcuts } from '../../import-export.helpers';
import { FormattingSettings } from '@/features/personalize/formatting-rules/types';
import { fetchMedicalChanges, isMedicalCategory, pickMedicalChanges } from '../../medical-changes/medical-changes';

interface PreloadedData {
    allSettings: AppSettings | null;
}

export const useExport = () => {
    const [isExporting, setIsExporting] = useState(false);
    const { t } = useTranslation();

    const handleExport = async (
        selectedCategories: CategoryKey[],
        selection: CategorySelection | undefined,
        preloaded: PreloadedData
    ) => {
        if (selectedCategories.length === 0) {
            return;
        }

        setIsExporting(true);

        try {
            const [allSettings, appVersion] = await Promise.all([
                preloaded.allSettings == null
                    ? invoke<AppSettings>('get_all_settings')
                    : Promise.resolve(preloaded.allSettings),
                getVersion(),
            ]);

            const categories: ExportedCategories = {};

            const fetchPromises: Promise<void>[] = [];

            const getSubItems = (key: CategoryKey) => selection?.[key]?.subItems;

            if (selectedCategories.includes('settings')) {
                categories.settings = extractSystemSettings(allSettings);
            }

            if (selectedCategories.includes('shortcuts')) {
                categories.shortcuts = extractShortcuts(allSettings);
            }

            if (selectedCategories.includes('formatting_rules')) {
                fetchPromises.push(
                    invoke<FormattingSettings>('get_formatting_settings').then((data) => {
                        const subItems = getSubItems('formatting_rules');
                        const includeBuiltIn = subItems?.['built_in'] !== false;
                        const filteredRules = data.rules.filter((rule) => {
                            if (subItems == null) {
                                return true;
                            }
                            return subItems[SUB_ITEM_KEY.rule(rule.id)] !== false;
                        });

                        categories.formatting_rules = {
                            built_in: includeBuiltIn ? data.built_in : undefined,
                            rules: filteredRules,
                        };
                    })
                );
            }

            if (selectedCategories.includes('dictionary')) {
                fetchPromises.push(
                    invoke<string[]>('get_dictionary').then((words) => {
                        const subItems = getSubItems('dictionary');
                        if (subItems == null) {
                            categories.dictionary = words;
                        } else {
                            categories.dictionary = words.filter((word) => subItems[SUB_ITEM_KEY.word(word)] !== false);
                        }
                    })
                );
            }

            // Plumed fork: the user's changes to the selected Corrections lists.
            if (selectedCategories.some(isMedicalCategory)) {
                fetchPromises.push(
                    fetchMedicalChanges().then((changes) => {
                        Object.assign(categories, pickMedicalChanges(changes, selectedCategories));
                    })
                );
            }

            await Promise.all(fetchPromises);

            const configFile: MurmureExportData = {
                version: CURRENT_MURMURE_FORMAT_VERSION,
                app_version: appVersion,
                exported_at: new Date().toISOString(),
                categories,
            };

            const content = JSON.stringify(configFile, null, 2);

            const today = new Date().toISOString().slice(0, 10);
            const filePath = await save({
                title: t('Export Configuration'),
                filters: [
                    {
                        name: 'Plumed Config',
                        extensions: ['murmure'],
                    },
                ],
                defaultPath: `plumed-config-${today}.murmure`,
            });

            if (filePath == null) {
                setIsExporting(false);
                return;
            }

            await invoke('write_murmure_file', { filePath, content });

            toast.success(t('Configuration exported to {{path}}.', { path: filePath }), { autoClose: 3000 });
        } catch (error) {
            toast.error(t('Failed to export configuration.') + ': ' + String(error));
        } finally {
            setIsExporting(false);
        }
    };

    return {
        isExporting,
        handleExport,
    };
};
