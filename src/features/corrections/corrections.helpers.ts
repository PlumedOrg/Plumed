export const CORRECTIONS_PATH = '/corrections';

export type CorrectionsTab = 'explanation' | 'dictionary' | 'rules';

/** Where a link of the Explanation tab leads. */
export interface CorrectionsTarget {
    tab: CorrectionsTab;
    /** Id of the block of the Rules tab to scroll to. */
    anchor?: string;
    /** The block is only listed with "Show the built-in rules" on. */
    builtIn?: boolean;
}

/** Ids of the Rules tab blocks the Explanation tab links to. */
export const RULES_ANCHORS = {
    userRules: 'corrections-user-rules',
    medicalRules: 'corrections-medical-rules',
    builtInOptions: 'corrections-built-in-options',
    medicalFixes: 'corrections-medical-fixes',
    phraseBoosting: 'corrections-phrase-boosting',
    confusionRescoring: 'corrections-confusion-rescoring',
    shortText: 'corrections-short-text',
    formattingOptions: 'corrections-formatting-options',
    hesitations: 'corrections-hesitations',
    medicalData: 'corrections-medical-data',
} as const;

/** The tab a URL hash opens. `custom-rules` is the hash of the old formatting rules page. */
export const tabFromHash = (hash: string): CorrectionsTab => {
    switch (hash) {
        case 'dictionary':
            return 'dictionary';
        case 'rules':
        case 'custom-rules':
            return 'rules';
        default:
            return 'explanation';
    }
};
