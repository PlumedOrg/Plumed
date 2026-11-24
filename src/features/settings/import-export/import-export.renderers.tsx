import { FormattingRulesSubItems } from './formatting-rules-sub-items/formatting-rules-sub-items';
import { SelectableWordList } from './selectable-word-list/selectable-word-list';
import { MedicalChangesReview, changeLines } from './medical-changes-review/medical-changes-review';
import { MEDICAL_CATEGORY_KEYS, MedicalChanges } from './medical-changes/medical-changes';
import { CategoryKey, DynamicSubItemsRenderer } from './import-export.types';
import { FormattingRule } from '@/features/personalize/formatting-rules/types';

export const buildRenderers = (
    rules: FormattingRule[],
    words: string[],
    medical: MedicalChanges = {}
): Partial<Record<CategoryKey, DynamicSubItemsRenderer>> => {
    const renderers: Partial<Record<CategoryKey, DynamicSubItemsRenderer>> = {};
    if (rules.length > 0) {
        renderers.formatting_rules = (props) => <FormattingRulesSubItems rules={rules} {...props} />;
    }
    if (words.length > 0) {
        renderers.dictionary = (props) => <SelectableWordList words={words} {...props} />;
    }
    for (const key of MEDICAL_CATEGORY_KEYS) {
        const lines = changeLines(key, medical);
        if (lines.length > 0) {
            renderers[key] = () => <MedicalChangesReview lines={lines} />;
        }
    }
    return renderers;
};
