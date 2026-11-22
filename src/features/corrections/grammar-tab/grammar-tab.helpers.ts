import { GrammarRule } from './grammar-tab.types';

type Translate = (key: string, options?: Record<string, string | number>) => string;

/** Rules whose state differs from the default: the user's changes. */
export const changeCount = (rules: GrammarRule[]): number =>
    rules.filter((rule) => rule.enabled !== rule.default).length;

/** The ids on after turning `id` on or off, e.g. for the tab's switches. */
export const enabledIdsWith = (rules: GrammarRule[], id: string, enabled: boolean): string[] =>
    rules.filter((rule) => (rule.id === id ? enabled : rule.enabled)).map((rule) => rule.id);

/** Rules whose id contains `query` (case ignored), all of them for an empty query. */
export const filterRules = (rules: GrammarRule[], query: string): GrammarRule[] => {
    const needle = query.trim().toLowerCase();
    return needle.length === 0 ? rules : rules.filter((rule) => rule.id.toLowerCase().includes(needle));
};

/**
 * "87 % right (45 better, 7 worse out of 53)", or a note when the rule never
 * fired with a single suggestion on the evaluation data.
 */
export const measureLabel = (rule: GrammarRule, t: Translate): string =>
    rule.precision === null
        ? t('Not seen on the evaluation data')
        : t('{{precision}} % right ({{better}} better, {{worse}} worse out of {{n}})', {
              precision: Math.round(rule.precision * 100),
              better: rule.better,
              worse: rule.worse,
              n: rule.n,
          });

/** Precision under this is shown as risky. */
export const RISKY_PRECISION = 0.8;

export const isRisky = (rule: GrammarRule): boolean => rule.precision !== null && rule.precision < RISKY_PRECISION;
