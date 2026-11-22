import { invoke } from '@tauri-apps/api/core';
import { AudioLines, ListChecks, SpellCheck, Stethoscope } from 'lucide-react';
import { CORRECTIONS_TAB_LABELS } from '@/features/corrections/corrections.helpers';
import { ConfusionGroup, ListOverlay } from '@/features/corrections/corrections.types';
import { MedicalRuleEdit } from '@/features/personalize/medical-rules/types';
import type { CategoryDefinition, CategorySelection, ImportStrategy } from '../import-export.types';

// Plumed fork (written with Claude Code): the user's changes to the lists of
// the Corrections page (the three medical lists and the grammar rules) travel in the `.murmure` export as
// one category each. Only the changes are exported (disabled bundled entries
// and added ones, or the rule edits), never the bundled lists. The backend
// (`medical/changes.rs`) validates and saves them like the tabs do, for this
// import and for `plumed import` alike. Files from before these categories
// simply lack them.

export const MEDICAL_CATEGORY_KEYS = [
    'medical_phrases',
    'medical_confusion_groups',
    'medical_rules',
    'grammar_rules',
] as const;

export type MedicalCategoryKey = (typeof MEDICAL_CATEGORY_KEYS)[number];

/** Rust `medical::MedicalChanges`: a key is absent when the file does not carry that list. */
export interface MedicalChanges {
    medical_phrases?: ListOverlay<string>;
    medical_confusion_groups?: ListOverlay<ConfusionGroup>;
    medical_rules?: MedicalRuleEdit[];
    /** Ids of the default grammar rules turned off and of the other rules turned on. */
    grammar_rules?: ListOverlay<string>;
}

export const isMedicalCategory = (key: string): key is MedicalCategoryKey =>
    MEDICAL_CATEGORY_KEYS.some((medicalKey) => medicalKey === key);

/** Named after the Corrections tab of each list, so both read the same. */
export const MEDICAL_CATEGORY_DEFINITIONS: CategoryDefinition[] = [
    {
        key: 'medical_phrases',
        label: CORRECTIONS_TAB_LABELS.vocabulary,
        icon: Stethoscope,
        supportsMerge: true,
        subItems: [],
    },
    {
        key: 'medical_confusion_groups',
        label: CORRECTIONS_TAB_LABELS['sound-alikes'],
        icon: AudioLines,
        supportsMerge: true,
        subItems: [],
    },
    {
        key: 'medical_rules',
        label: CORRECTIONS_TAB_LABELS['model-corrections'],
        icon: ListChecks,
        supportsMerge: true,
        subItems: [],
    },
    {
        key: 'grammar_rules',
        label: CORRECTIONS_TAB_LABELS.grammar,
        icon: SpellCheck,
        supportsMerge: true,
        subItems: [],
    },
];

/** Progress line of the import modal (English keys of `fr.json`). */
export const MEDICAL_IMPORT_STEP_LABELS: Record<MedicalCategoryKey, string> = {
    medical_phrases: 'Importing medical vocabulary changes...',
    medical_confusion_groups: 'Importing sound-alike word changes...',
    medical_rules: 'Importing residual error changes...',
    grammar_rules: 'Importing grammar rule changes...',
};

/** Every medical category selected, the export default. */
export const medicalInitialSelection = (): CategorySelection =>
    Object.fromEntries(MEDICAL_CATEGORY_KEYS.map((key) => [key, { selected: true, subItems: {} }]));

/**
 * The lists of `changes` named in `keys` (other category keys are ignored).
 * Example: pickMedicalChanges({ medical_phrases: p, medical_rules: r }, ['medical_rules', 'dictionary'])
 * gives { medical_rules: r }.
 */
export const pickMedicalChanges = (changes: MedicalChanges, keys: readonly string[]): MedicalChanges => {
    const picked: MedicalChanges = {};
    if (keys.includes('medical_phrases') && changes.medical_phrases != null) {
        picked.medical_phrases = changes.medical_phrases;
    }
    if (keys.includes('medical_confusion_groups') && changes.medical_confusion_groups != null) {
        picked.medical_confusion_groups = changes.medical_confusion_groups;
    }
    if (keys.includes('medical_rules') && changes.medical_rules != null) {
        picked.medical_rules = changes.medical_rules;
    }
    if (keys.includes('grammar_rules') && changes.grammar_rules != null) {
        picked.grammar_rules = changes.grammar_rules;
    }
    return picked;
};

const overlaySize = <T>(overlay: ListOverlay<T>): number => overlay.disabled.length + overlay.added.length;

/** How many changes each list carries, shown next to its category. */
export const medicalCounters = (changes: MedicalChanges): Partial<Record<MedicalCategoryKey, number>> => {
    const counters: Partial<Record<MedicalCategoryKey, number>> = {};
    if (changes.medical_phrases != null) {
        counters.medical_phrases = overlaySize(changes.medical_phrases);
    }
    if (changes.medical_confusion_groups != null) {
        counters.medical_confusion_groups = overlaySize(changes.medical_confusion_groups);
    }
    if (changes.medical_rules != null) {
        counters.medical_rules = changes.medical_rules.length;
    }
    if (changes.grammar_rules != null) {
        counters.grammar_rules = overlaySize(changes.grammar_rules);
    }
    return counters;
};

export const fetchMedicalChanges = (): Promise<MedicalChanges> => invoke<MedicalChanges>('get_medical_changes');

/** Saves one imported list, replacing or merged into the user's changes. */
export const applyMedicalCategory = async (
    key: MedicalCategoryKey,
    changes: MedicalChanges,
    strategy: ImportStrategy
): Promise<void> => {
    const list = pickMedicalChanges(changes, [key]);
    await invoke('import_medical_changes', { changes: list, merge: strategy === 'merge' });
};
