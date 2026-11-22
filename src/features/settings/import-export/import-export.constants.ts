import { Settings, Keyboard, AlignLeft, BookText } from 'lucide-react';
import { CategoryDefinition } from './import-export.types';
import { CORRECTIONS_TAB_LABELS } from '@/features/corrections/corrections.helpers';
import { MEDICAL_CATEGORY_DEFINITIONS } from './medical-changes/medical-changes';

export const CURRENT_MURMURE_FORMAT_VERSION = 1;

export const DICTIONARY_PREVIEW_LIMIT = 15;

export const SUB_ITEM_KEY = {
    rule: (id: string) => `rule_${id}`,
    word: (word: string) => `word_${word}`,
} as const;

export const CATEGORY_DEFINITIONS: CategoryDefinition[] = [
    {
        key: 'settings',
        label: 'System Settings',
        icon: Settings,
        supportsMerge: false,
        subItems: [],
    },
    {
        key: 'shortcuts',
        label: 'Shortcuts',
        icon: Keyboard,
        supportsMerge: false,
        subItems: [],
    },
    // Plumed fork: named after the Corrections tabs (formatting rules, with
    // the options of the Clean-up tab as sub-item; personal dictionary).
    {
        key: 'formatting_rules',
        label: CORRECTIONS_TAB_LABELS.formatting,
        icon: AlignLeft,
        supportsMerge: true,
        subItems: [{ key: 'built_in', label: 'Options of the Clean-up and standardization tab' }],
    },
    {
        key: 'dictionary',
        label: CORRECTIONS_TAB_LABELS.dictionary,
        icon: BookText,
        supportsMerge: true,
        subItems: [],
    },
    ...MEDICAL_CATEGORY_DEFINITIONS,
];
