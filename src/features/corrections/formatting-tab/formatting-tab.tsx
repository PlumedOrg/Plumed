import { invoke } from '@tauri-apps/api/core';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { FormattingRules } from '@/features/personalize/formatting-rules/formatting-rules';
import { useFormattingRules } from '@/features/personalize/formatting-rules/hooks/use-formatting-rules';
import { FormattingRule } from '@/features/personalize/formatting-rules/types';
import { useFormattingRulesFile } from '../hooks/use-formatting-rules-file';
import { useShowBuiltInRules } from '../hooks/use-show-built-in-rules';
import { ListHeader } from '../list-header/list-header';

interface FormattingTabProps {
    /** Id of the block to scroll to once loaded. */
    anchor: string | null;
}

/** "Formatting rules" tab: the user's rules (seeded with InterHop's) and the pre-installed options. */
export const FormattingTab = ({ anchor }: FormattingTabProps) => {
    const { t } = useTranslation();
    const formatting = useFormattingRules();
    const { settings, saveSettings, loadSettings } = formatting;
    const [showBuiltIn, setShowBuiltIn] = useShowBuiltInRules();
    const { exportRules, importRules } = useFormattingRulesFile(loadSettings);

    const resetRules = async () => {
        try {
            const rules = await invoke<FormattingRule[]>('get_default_formatting_rules');
            await saveSettings({ ...settings, rules });
        } catch (error) {
            toast.error(String(error));
        }
    };

    return (
        <div className="space-y-6">
            <ListHeader
                title={t('Your formatting rules')}
                description={t(
                    'Your own find and replace rules, applied from top to bottom. A fresh install starts with the medical dictation rules of InterHop, which you can edit, turn off or delete.'
                )}
                summary={t('{{count}} rule(s)', { count: settings.rules.length })}
                resetDescription={t(
                    'Your formatting rules are replaced by the InterHop rules a fresh install starts with. The options stay as they are. This cannot be undone.'
                )}
                onReset={resetRules}
                onExport={exportRules}
                onImport={importRules}
                testId="formatting-rules-list"
            />
            <FormattingRules
                formatting={formatting}
                showBuiltIn={showBuiltIn}
                onShowBuiltInChange={setShowBuiltIn}
                anchor={anchor}
            />
        </div>
    );
};
