/** How a catalog id matches Grammalecte's rule ids (Rust `grammar::rules::RuleLevel`). */
export type GrammarRuleLevel = 'family' | 'rule';

/**
 * One rule of the grammar step and what it did on the UltiMed dev set, masks on
 * (Rust `grammar::RuleView`): `n` fixes with one suggestion, `better` brought the
 * text closer to the reference, `worse` further; `precision` is null when it
 * never fired with one suggestion.
 */
export interface GrammarRule {
    id: string;
    level: GrammarRuleLevel;
    n: number;
    better: number;
    worse: number;
    precision: number | null;
    /** On unless the user turns it off. */
    default: boolean;
    enabled: boolean;
}

/** Rust `grammar::GrammarRules`. */
export interface GrammarRules {
    grammalecte_version: string;
    rules: GrammarRule[];
}
