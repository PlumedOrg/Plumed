export type MatchMode = 'smart' | 'exact' | 'regex';

export interface FormattingRule {
    id: string;
    trigger: string;
    replacement: string;
    enabled: boolean;
    match_mode: MatchMode;
    name?: string;
}

export interface BuiltInOptions {
    short_text_correction: number;
    space_before_punctuation: boolean;
    trailing_space: boolean;
    convert_text_numbers: boolean;
    text_numbers_language: string;
    text_numbers_threshold: number;
    medical_fixes: boolean;
    phrase_boosting: boolean;
    /** Replaces the phrase list's default weight, `null` keeps the list's own. */
    phrase_boost_weight: number | null;
    /** Replaces the phrase list's default min-p (margin), `null` keeps the list's own. */
    phrase_boost_minp: number | null;
    confusion_rescoring: boolean;
    grammar: boolean;
}

export interface FormattingSettings {
    built_in: BuiltInOptions;
    rules: FormattingRule[];
}

export const defaultFormattingSettings: FormattingSettings = {
    built_in: {
        short_text_correction: 3,
        // Plumed: French typography and digits from 2 up by default
        space_before_punctuation: true,
        trailing_space: true,
        convert_text_numbers: true,
        text_numbers_language: 'fr',
        text_numbers_threshold: 2,
        medical_fixes: true,
        phrase_boosting: true,
        phrase_boost_weight: null,
        phrase_boost_minp: null,
        confusion_rescoring: true,
        grammar: true,
    },
    rules: [],
};

export function migrateRule(raw: Record<string, unknown>): FormattingRule {
    if (typeof raw.match_mode === 'string') {
        return raw as unknown as FormattingRule;
    }
    return {
        id: raw.id as string,
        trigger: raw.trigger as string,
        replacement: raw.replacement as string,
        enabled: raw.enabled as boolean,
        match_mode: raw.exact_match === true ? 'exact' : 'smart',
        name: raw.name as string | undefined,
    };
}
