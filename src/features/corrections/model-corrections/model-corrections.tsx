import { useTranslation } from '@/i18n';
import { useFormattingRules } from '@/features/personalize/formatting-rules/hooks/use-formatting-rules';
import { useMedicalRules } from '@/features/personalize/medical-rules/hooks/use-medical-rules';
import { MedicalRules } from '@/features/personalize/medical-rules/medical-rules';
import { useListFile } from '../hooks/use-list-file';
import { ListHeader } from '../list-header/list-header';

/** "Model corrections" tab: the UltiMed and Plumed fix rules, with the user's changes. */
export const ModelCorrections = () => {
    const { t } = useTranslation();
    const { settings, updateBuiltInOption } = useFormattingRules();
    const { edits, saveEdits, reload } = useMedicalRules();
    const { exportList, importList } = useListFile('rules', reload);

    return (
        <div className="space-y-6">
            <ListHeader
                title={t('Fix frequent model errors')}
                description={t(
                    "About 25,000 rules rewrite what the model often gets wrong on drug names and medical terms: UltiMed's rules, then Plumed's own. Your changes come on top: an added rule runs before the others, a changed or turned off rule replaces its pre-installed version."
                )}
                option={{
                    checked: settings.built_in.medical_fixes,
                    onChange: (checked) => updateBuiltInOption('medical_fixes', checked),
                    testId: 'option-medical-fixes',
                }}
                summary={edits.length}
                resetDescription={t(
                    'Your added and changed rules are deleted and every pre-installed rule is turned back on. This cannot be undone.'
                )}
                onReset={() => saveEdits([])}
                onExport={exportList}
                onImport={importList}
                testId="model-corrections"
            />
            <MedicalRules edits={edits} saveEdits={saveEdits} />
        </div>
    );
};
