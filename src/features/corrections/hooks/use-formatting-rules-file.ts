import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { FormattingRule } from '@/features/personalize/formatting-rules/types';
import { useExport } from '@/features/settings/import-export/export-section/hooks/use-export';
import { applySingleCategory } from '@/features/settings/import-export/import-section/import-section.helpers';
import { MurmureExportData } from '@/features/settings/import-export/import-export.types';

/** The rules of a `.murmure` file, or `null` when it has none. */
const rulesOf = (content: string): FormattingRule[] | null => {
    const parsed = JSON.parse(content) as Partial<MurmureExportData>;
    const rules = parsed.categories?.formatting_rules?.rules;
    return Array.isArray(rules) ? rules : null;
};

/** Regex rules that do not compile, as "trigger: error". */
const invalidRegexes = async (rules: FormattingRule[]): Promise<string[]> => {
    const errors = await Promise.all(
        rules
            .filter((rule) => rule.match_mode === 'regex')
            .map((rule) =>
                invoke('validate_regex', { pattern: rule.trigger }).then(
                    () => null,
                    (error: unknown) => `${rule.trigger}: ${String(error)}`
                )
            )
    );
    return errors.filter((error): error is string => error != null);
};

/**
 * Export and import of the user's formatting rules in the app's `.murmure`
 * settings format (the same file as Settings, with only the rules). An
 * import replaces the rules and keeps the options.
 */
export const useFormattingRulesFile = (onImported: () => void) => {
    const { t } = useTranslation();
    const { handleExport } = useExport();

    const exportRules = () => handleExport(['formatting_rules'], undefined, { allSettings: null });

    const importRules = async () => {
        try {
            const filePath = await open({
                title: t('Import a list'),
                multiple: false,
                directory: false,
                filters: [{ name: 'Plumed Config', extensions: ['murmure'] }],
            });
            if (filePath == null) {
                return;
            }
            const rules = rulesOf(await invoke<string>('read_murmure_file', { filePath }));
            if (rules == null) {
                throw new Error(t('The file has no formatting rule.'));
            }
            const errors = await invalidRegexes(rules);
            if (errors.length > 0) {
                throw new Error(errors.join('\n'));
            }
            await applySingleCategory('formatting_rules', { formatting_rules: { rules } }, {});
            onImported();
            toast.success(t('List imported'), { autoClose: 2000 });
        } catch (error) {
            toast.error(`${t('Nothing was imported')}: ${String(error)}`, { autoClose: false });
        }
    };

    return { exportRules, importRules };
};
