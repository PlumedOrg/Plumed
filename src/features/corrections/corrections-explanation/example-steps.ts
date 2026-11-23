import type { HistoryStep } from '@/features/home/history/history-steps/history-steps.helpers';

// Plumed fork (written with Claude Code): a hardcoded, realistic dictated
// sentence as each step corrects it, in the shape the history records so the
// Explanation tab can show it with the history component. Illustration only:
// no command runs the real pipeline without audio. French on purpose, like
// the dictation it illustrates (it is not translated).
export const EXAMPLE_STEPS: HistoryStep[] = [
    {
        step_id: 'recognition',
        text_after:
            'monsieur marten est suivi pour une hypertension arterielle il prend du doliprane mille milligrammes euh deux fois par jour et les douleurs sont soulagé',
    },
    {
        step_id: 'dictionary',
        text_after:
            'monsieur Martin est suivi pour une hypertension arterielle il prend du doliprane mille milligrammes euh deux fois par jour et les douleurs sont soulagé',
    },
    {
        step_id: 'clean_up',
        text_after:
            'monsieur Martin est suivi pour une hypertension arterielle il prend du doliprane mille milligrammes deux fois par jour et les douleurs sont soulagé',
    },
    {
        step_id: 'model_corrections',
        text_after:
            'monsieur Martin est suivi pour une hypertension artérielle il prend du Doliprane mille milligrammes deux fois par jour et les douleurs sont soulagé',
    },
    {
        step_id: 'formatting_rules',
        text_after:
            'monsieur Martin est suivi pour une hypertension artérielle il prend du Doliprane 1000 mg deux fois par jour et les douleurs sont soulagé',
    },
    {
        step_id: 'built_in_formatting',
        text_after:
            'Monsieur Martin est suivi pour une hypertension artérielle. Il prend du Doliprane 1000 mg deux fois par jour et les douleurs sont soulagé.',
    },
    {
        step_id: 'grammar',
        text_after:
            'Monsieur Martin est suivi pour une hypertension artérielle. Il prend du Doliprane 1000 mg deux fois par jour et les douleurs sont soulagées.',
    },
];
