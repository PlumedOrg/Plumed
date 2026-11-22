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
    | 'formatting';

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
 * The steps that change the text, in the order the code runs them
 * (`audio/pipeline.rs`, then `formatting_rules/formatter.rs`). Ids after
 * recognition match the Rust `history::PipelineStep` the history keeps.
 */
export type PipelineStepId =
    | 'vocabulary'
    | 'sound_alikes'
    | 'dictionary'
    | 'clean_up'
    | 'model_corrections'
    | 'short_text'
    | 'formatting_rules'
    | 'built_in_formatting';

/** Each step and the tab holding its list or options. */
export const PIPELINE_STEPS: { id: PipelineStepId; tab: CorrectionsTab }[] = [
    { id: 'vocabulary', tab: 'vocabulary' },
    { id: 'sound_alikes', tab: 'sound-alikes' },
    { id: 'dictionary', tab: 'dictionary' },
    { id: 'clean_up', tab: 'clean-up' },
    { id: 'model_corrections', tab: 'model-corrections' },
    { id: 'short_text', tab: 'clean-up' },
    { id: 'formatting_rules', tab: 'formatting' },
    { id: 'built_in_formatting', tab: 'clean-up' },
];

/** Number of a step in the Explanation flow, starting at 1. */
export const stepNumber = (id: PipelineStepId): number => PIPELINE_STEPS.findIndex((step) => step.id === id) + 1;

/** "(4, 6, 8) Label": a tab label prefixed with the numbers of the steps it covers. */
export const numberedTabLabel = (tab: CorrectionsTab, label: string): string => {
    const numbers = PIPELINE_STEPS.flatMap((step, index) => (step.tab === tab ? [index + 1] : []));
    return numbers.length > 0 ? `(${numbers.join(', ')}) ${label}` : label;
};

type Translate = (key: string, options?: Record<string, string | number>) => string;

/** Title of a step, shared by the Explanation tab and the history details. */
export const pipelineStepTitle = (id: PipelineStepId, t: Translate): string => {
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
            return t('Fix frequent model errors');
        case 'short_text':
            return t('Short text correction');
        case 'formatting_rules':
            return t('Your formatting rules');
        case 'built_in_formatting':
            return t('Pre-installed formatting');
    }
};

/** "(4) Clean-up and standardization": a step title with its number. */
export const numberedStepTitle = (id: PipelineStepId, t: Translate): string =>
    `(${stepNumber(id)}) ${pipelineStepTitle(id, t)}`;

const TABS: CorrectionsTab[] = [
    'explanation',
    'vocabulary',
    'sound-alikes',
    'dictionary',
    'clean-up',
    'model-corrections',
    'formatting',
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
            return pickTabFromHash(hash, TABS, 'explanation');
    }
};
