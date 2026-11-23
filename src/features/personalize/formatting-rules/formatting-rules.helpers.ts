import { FormattingRule } from './types';

/** Rules whose name, trigger or replacement contains the query (case-insensitive); all of them for an empty query. */
export const filterFormattingRules = (rules: FormattingRule[], query: string): FormattingRule[] => {
    const needle = query.trim().toLowerCase();
    if (needle === '') {
        return rules;
    }
    return rules.filter((rule) =>
        [rule.name ?? '', rule.trigger, rule.replacement].some((field) => field.toLowerCase().includes(needle))
    );
};
