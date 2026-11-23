import { pickTabFromHash } from '@/components/tabs.helpers';

export const CORRECTIONS_PATH = '/corrections';

/** One tab per list, in the order the pipeline uses them, after the explanation. */
export type CorrectionsTab =
    | 'explanation'
    | 'vocabulary'
    | 'sound-alikes'
    | 'dictionary'
    | 'clean-up'
    | 'model-corrections'
    | 'formatting'
    | 'grammar';

/** Where a button of the Explanation tab leads. */
export interface CorrectionsTarget {
    tab: CorrectionsTab;
    /** Id of the block of the Clean-up tab to scroll to. */
    anchor?: string;
}

/** Ids of the Clean-up tab blocks the Explanation tab links to. */
export const RULES_ANCHORS = {
    shortText: 'corrections-short-text',
    formattingOptions: 'corrections-formatting-options',
    hesitations: 'corrections-hesitations',
} as const;

/**
 * The steps of the Explanation flow, in the order the code runs them
 * (`audio/pipeline.rs`, then `formatting_rules/formatter.rs`, then
 * `grammar/`), one per list tab. Ids after recognition match the Rust `history::PipelineStep` the
 * history keeps.
 */
export type PipelineStepId =
    | 'vocabulary'
    | 'sound_alikes'
    | 'dictionary'
    | 'clean_up'
    | 'model_corrections'
    | 'formatting_rules'
    | 'grammar';

/**
 * Options of the Clean-up tab that the code runs later than step 4. They are
 * not steps of their own: the Explanation card, the Clean-up tab and the
 * history show them inside step 4, saying when they run.
 */
export type CleanUpOptionId = 'short_text' | 'built_in_formatting';

/** Anything the history records a text for after recognition. */
export type TextStepId = PipelineStepId | CleanUpOptionId;

/** Each step and the tab holding its list or options. Step numbers come from this order. */
export const PIPELINE_STEPS: { id: PipelineStepId; tab: CorrectionsTab }[] = [
    { id: 'vocabulary', tab: 'vocabulary' },
    { id: 'sound_alikes', tab: 'sound-alikes' },
    { id: 'dictionary', tab: 'dictionary' },
    { id: 'clean_up', tab: 'clean-up' },
    { id: 'model_corrections', tab: 'model-corrections' },
    { id: 'formatting_rules', tab: 'formatting' },
    { id: 'grammar', tab: 'grammar' },
];

/**
 * The step after which each option of the Clean-up tab runs: short text
 * correction after the residual errors (step 5), the pre-installed
 * formatting last, after the user's formatting rules (step 6).
 */
export const CLEAN_UP_OPTIONS: { id: CleanUpOptionId; runsAfter: PipelineStepId }[] = [
    { id: 'short_text', runsAfter: 'model_corrections' },
    { id: 'built_in_formatting', runsAfter: 'formatting_rules' },
];

const isCleanUpOption = (id: TextStepId): id is CleanUpOptionId => CLEAN_UP_OPTIONS.some((option) => option.id === id);

/** Number of a step in the Explanation flow, starting at 1. Clean-up options get the number of step 4. */
export const stepNumber = (id: TextStepId): number => {
    const stepId: PipelineStepId = isCleanUpOption(id) ? 'clean_up' : id;
    return PIPELINE_STEPS.findIndex((step) => step.id === stepId) + 1;
};

/** Number of the step a Clean-up option runs after, `null` for a real step. */
export const runsAfterStep = (id: TextStepId): number | null => {
    const option = CLEAN_UP_OPTIONS.find((candidate) => candidate.id === id);
    return option === undefined ? null : stepNumber(option.runsAfter);
};

/** Number of the step a tab holds, `null` for the Explanation tab. */
export const tabStepNumber = (tab: CorrectionsTab): number | null => {
    const index = PIPELINE_STEPS.findIndex((step) => step.tab === tab);
    return index >= 0 ? index + 1 : null;
};

/**
 * Tab names (English keys of `fr.json`, translate them with `t`), shared by
 * the tabs and the categories of the settings import and export, so both
 * read the same.
 */
export const CORRECTIONS_TAB_LABELS: Record<CorrectionsTab, string> = {
    explanation: 'Explanations',
    vocabulary: 'Medical vocabulary',
    'sound-alikes': 'Sound-alike words',
    dictionary: 'Personal dictionary',
    'clean-up': 'Clean-up and standardization',
    'model-corrections': 'Residual errors',
    formatting: 'Formatting',
    grammar: 'Grammar',
};

type Translate = (key: string, options?: Record<string, string | number>) => string;

/** Title of a step or Clean-up option, shared by the Explanation tab, the Clean-up tab and the history details. */
export const pipelineStepTitle = (id: TextStepId, t: Translate): string => {
    switch (id) {
        case 'vocabulary':
            return t('Boost medical vocabulary');
        case 'sound_alikes':
            return t('Tell apart words that sound alike');
        case 'dictionary':
            return t('Restore the spelling of the personal dictionary');
        case 'clean_up':
            return t('Clean-up and standardization');
        case 'model_corrections':
            return t('Fix the residual errors of the model');
        case 'short_text':
            return t('Short text correction');
        case 'formatting_rules':
            return t('Your formatting rules');
        case 'built_in_formatting':
            return t('Pre-installed formatting');
        case 'grammar':
            return t('Fix the grammar');
    }
};

/**
 * Name of a step as its Corrections tab shows it ("Residual errors"), the
 * history uses it. A Clean-up option has no tab of its own, so it keeps its title.
 */
export const stepName = (id: TextStepId, t: Translate): string => {
    const tab = PIPELINE_STEPS.find((step) => step.id === id)?.tab;
    return tab === undefined ? pipelineStepTitle(id, t) : t(CORRECTIONS_TAB_LABELS[tab]);
};

/** "applied after step 5":when a Clean-up option runs, `null` for a real step. */
export const cleanUpOptionTiming = (id: TextStepId, t: Translate): string | null => {
    const after = runsAfterStep(id);
    return after === null ? null : t('applied after step {{step}}', { step: after });
};

/** The tabs in the order they are shown. */
export const CORRECTIONS_TABS: CorrectionsTab[] = [
    'explanation',
    'vocabulary',
    'sound-alikes',
    'dictionary',
    'clean-up',
    'model-corrections',
    'formatting',
    'grammar',
];

/**
 * The tab a URL hash opens. `rules` (the former Rules tab) and `custom-rules`
 * (the hash of the old formatting rules page) open the Formatting tab.
 */
export const tabFromHash = (hash: string): CorrectionsTab => {
    switch (hash) {
        case 'rules':
        case 'custom-rules':
            return 'formatting';
        default:
            return pickTabFromHash(hash, CORRECTIONS_TABS, 'explanation');
    }
};
