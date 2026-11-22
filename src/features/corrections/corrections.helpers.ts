export const CORRECTIONS_PATH = '/corrections';

/** One tab per list, in the order the pipeline uses them, after the explanation. */
export type CorrectionsTab =
    | 'explanation'
    | 'vocabulary'
    | 'sound-alikes'
    | 'dictionary'
    | 'model-corrections'
    | 'formatting';

/** Where a button of the Explanation tab leads. */
export interface CorrectionsTarget {
    tab: CorrectionsTab;
    /** Id of the block of the Formatting tab to scroll to. */
    anchor?: string;
    /** The block is only listed with "Show the pre-installed rules" on. */
    builtIn?: boolean;
}

/** Ids of the Formatting tab blocks the Explanation tab links to. */
export const RULES_ANCHORS = {
    userRules: 'corrections-user-rules',
    builtInOptions: 'corrections-built-in-options',
    shortText: 'corrections-short-text',
    formattingOptions: 'corrections-formatting-options',
    hesitations: 'corrections-hesitations',
} as const;

const TABS: CorrectionsTab[] = [
    'explanation',
    'vocabulary',
    'sound-alikes',
    'dictionary',
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
            return TABS.find((tab) => tab === hash) ?? 'explanation';
    }
};
