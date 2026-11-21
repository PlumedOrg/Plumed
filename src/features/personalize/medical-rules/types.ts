/** A user change to the bundled medical rules (Rust `medical::RuleEdit`). */
export interface MedicalRuleEdit {
    /** Full pattern of the bundled rule this edit replaces, or null for a new rule. */
    original: string | null;
    /** What to match, without word boundaries (always added). Case-insensitive. */
    core: string;
    replacement: string;
    enabled: boolean;
}

/**
 * 'added' rules are deleted, 'changed' ones reverted to the bundled rule,
 * 'orphaned' ones (edits of a rule the medical data in use no longer has) deleted.
 */
export type MedicalRuleStatus = 'bundled' | 'changed' | 'added' | 'orphaned';

/** A bundled medical rule (Rust `medical::RuleView`). */
export interface MedicalRuleView {
    original: string;
    core: string;
    replacement: string;
}
