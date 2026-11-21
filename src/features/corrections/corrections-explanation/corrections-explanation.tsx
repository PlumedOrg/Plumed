import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@/components/button';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { useMedicalDataStatus } from '@/features/medical-data/hooks/use-medical-data-status';
import { medicalDataVersionLabel } from '@/features/medical-data/medical-data.helpers';
import { useFormattingRules } from '@/features/personalize/formatting-rules/hooks/use-formatting-rules';
import { useRemoveHesitations } from '@/features/personalize/formatting-rules/hooks/use-remove-hesitations';
import { useMedicalRules } from '@/features/personalize/medical-rules/hooks/use-medical-rules';
import { CorrectionsTarget, RULES_ANCHORS } from '../corrections.helpers';
import { PipelineStep, StepAction, StepStatus } from './pipeline-step/pipeline-step';

interface CorrectionsExplanationProps {
    onOpen: (target: CorrectionsTarget) => void;
}

interface Step {
    title: string;
    stage: string;
    description: string;
    example?: string;
    note?: string;
    statuses: StepStatus[];
    actions: StepAction[];
}

/** Explanation tab: every list that changes the text, in the order the code applies them. */
export const CorrectionsExplanation = ({ onOpen }: CorrectionsExplanationProps) => {
    const { t } = useTranslation();
    const { settings, isLoading } = useFormattingRules();
    const { removeHesitations } = useRemoveHesitations();
    const { edits } = useMedicalRules();
    const { status: dataStatus } = useMedicalDataStatus();
    const [dictionaryCount, setDictionaryCount] = useState(0);

    useEffect(() => {
        invoke<string[]>('get_dictionary')
            .then((words) => setDictionaryCount(words.length))
            .catch((e) => console.error('Failed to load the dictionary:', e));
    }, []);

    if (isLoading) {
        return <div className="text-muted-foreground">{t('Loading...')}</div>;
    }

    const options = settings.built_in;
    const onOff = (active: boolean): StepStatus => ({ label: active ? t('On') : t('Off'), active });
    const dictionaryStatus: StepStatus = {
        label: t('{{count}} dictionary word(s)', { count: dictionaryCount }),
        active: dictionaryCount > 0,
    };
    const enabledRules = settings.rules.filter((rule) => rule.enabled).length;
    const openOption = (anchor: string, testId: string): StepAction => ({
        label: t('Open the option'),
        onClick: () => onOpen({ tab: 'rules', anchor, builtIn: true }),
        testId,
    });
    const openDictionary = (testId: string): StepAction => ({
        label: t('Open the dictionary'),
        onClick: () => onOpen({ tab: 'dictionary' }),
        testId,
    });

    const steps: Step[] = [
        {
            title: t('Boost medical vocabulary'),
            stage: t('During recognition'),
            description: t(
                'The recognition favors about 77,000 French medical words and phrases from the bundled list, plus the words of your personal dictionary.'
            ),
            statuses: [onOff(options.phrase_boosting), dictionaryStatus],
            actions: [
                openOption(RULES_ANCHORS.phraseBoosting, 'corrections-open-phrase-boosting'),
                openDictionary('corrections-open-dictionary-boost'),
            ],
        },
        {
            title: t('Resolve sound-alike medical words'),
            stage: t('Second pass on the audio'),
            description: t(
                'About 140 groups of medical words that sound alike, from UltiMed. When the recognized text contains one of them, Plumed listens to that passage again and keeps the word the audio supports. Final transcriptions only.'
            ),
            example: t('Example: "singulaire" → "Singulair" or "cingulaire"'),
            statuses: [onOff(options.confusion_rescoring)],
            actions: [openOption(RULES_ANCHORS.confusionRescoring, 'corrections-open-confusion-rescoring')],
        },
        {
            title: t('Personal dictionary spelling'),
            stage: t('On the text'),
            description: t(
                'A recognized word very close to one of your dictionary words is written as in your dictionary (spelling, accents, capitals).'
            ),
            statuses: [dictionaryStatus],
            actions: [openDictionary('corrections-open-dictionary-spelling')],
        },
        {
            title: t('Clean-up'),
            stage: t('On the text'),
            description: t('Words repeated by mistake are removed, and hesitation sounds too when that option is on.'),
            example: t('Example: "I um think so" → "I think so"'),
            statuses: [onOff(removeHesitations)],
            actions: [openOption(RULES_ANCHORS.hesitations, 'corrections-open-hesitations')],
        },
        {
            title: t('Fix drug names and medical terms'),
            stage: t('On the text'),
            description: t(
                "About 25,000 UltiMed rules in six files, applied in this order: drugs, dictionary, PARHAF, PARROT, acronyms, then the hand-written file. Then Plumed's own rules file. Your changes come on top: an added rule runs before the bundled ones, a changed or disabled rule replaces its bundled version."
            ),
            example: t('Example: "tacadol" → "Takadol"'),
            note: t(
                'UltiMed also publishes the same rules as lookup tables, an equivalent form for apps whose regex engine lacks lookbehind. Plumed uses the regex form, so the lookup tables are not an extra list.'
            ),
            statuses: [
                onOff(options.medical_fixes),
                { label: t('{{count}} change(s)', { count: edits.length }), active: edits.length > 0 },
            ],
            actions: [
                ...(options.medical_fixes
                    ? [
                          {
                              label: t('Open your changes'),
                              onClick: () => onOpen({ tab: 'rules', anchor: RULES_ANCHORS.medicalRules }),
                              testId: 'corrections-open-medical-rules',
                          },
                      ]
                    : []),
                openOption(RULES_ANCHORS.medicalFixes, 'corrections-open-medical-fixes'),
            ],
        },
        {
            title: t('Short text correction'),
            stage: t('On the text'),
            description:
                options.short_text_correction > 0
                    ? t('Removes capitalization and punctuation for transcriptions of {{count}} word(s) or fewer.', {
                          count: options.short_text_correction,
                      })
                    : t('Removes capitalization and punctuation from very short transcriptions.'),
            example: t('Example: "Hello." → "hello"'),
            statuses: [onOff(options.short_text_correction > 0)],
            actions: [openOption(RULES_ANCHORS.shortText, 'corrections-open-short-text')],
        },
        {
            title: t('Your formatting rules'),
            stage: t('On the text'),
            description: t(
                "Your own find and replace rules, applied from top to bottom. A fresh install starts with InterHop's medical dictation rules, which you can edit, turn off or delete."
            ),
            example: t('Example: "semicolon" → ";"'),
            statuses: [{ label: t('{{count}} rule(s) on', { count: enabledRules }), active: enabledRules > 0 }],
            actions: [
                {
                    label: t('Open your rules'),
                    onClick: () => onOpen({ tab: 'rules', anchor: RULES_ANCHORS.userRules }),
                    testId: 'corrections-open-user-rules',
                },
            ],
        },
        {
            title: t('Built-in formatting'),
            stage: t('On the text'),
            description: t(
                'Last, the built-in options: space before ? and !, numbers written as digits, and a space at the end of each transcription.'
            ),
            statuses: [
                { label: t('Add space before ? and !'), active: options.space_before_punctuation },
                { label: t('Convert text numbers to digits'), active: options.convert_text_numbers },
                { label: t('Add space at end of transcription'), active: options.trailing_space },
            ],
            actions: [openOption(RULES_ANCHORS.formattingOptions, 'corrections-open-formatting-options')],
        },
    ];

    return (
        <div className="space-y-6" data-testid="corrections-explanation">
            <Typography.Paragraph>
                {t(
                    'Plumed corrects the transcribed text in several steps, always in this order. The badges show what is on, and the buttons open the settings of each step.'
                )}
            </Typography.Paragraph>

            <ol>
                {steps.map((step, index) => (
                    <PipelineStep
                        key={step.title}
                        index={index + 1}
                        title={step.title}
                        stage={step.stage}
                        statuses={step.statuses}
                        actions={step.actions}
                        isLast={index === steps.length - 1}
                    >
                        <Typography.Paragraph>
                            {step.description}
                            {step.example != null && <span className="block text-xs italic mt-1">{step.example}</span>}
                        </Typography.Paragraph>
                        {step.note != null && (
                            <p className="rounded-md border border-surface-border px-3 py-2 text-xs text-muted-foreground">
                                {step.note}
                            </p>
                        )}
                    </PipelineStep>
                ))}
            </ol>

            {dataStatus != null && (
                <SettingsUI.Container className="space-y-2 p-4" data-testid="corrections-medical-data">
                    <Typography.Title>{t('Medical data')}</Typography.Title>
                    <Typography.Paragraph>
                        {t(
                            'The phrase list (step 1), the sound-alike groups (step 2) and the drug and medical term rules (step 5) form the medical data.'
                        )}{' '}
                        {medicalDataVersionLabel(dataStatus, t)}
                    </Typography.Paragraph>
                    {dataStatus.available && (
                        <>
                            <Typography.Paragraph>
                                {t(
                                    'When a signed update is offered online, Plumed asks before downloading it. Your own changes are kept.'
                                )}
                            </Typography.Paragraph>
                            <Button
                                variant="secondary"
                                size="sm"
                                onClick={() =>
                                    onOpen({ tab: 'rules', anchor: RULES_ANCHORS.medicalData, builtIn: true })
                                }
                                data-testid="corrections-open-medical-data"
                            >
                                {t('Open the medical data updates')}
                            </Button>
                        </>
                    )}
                </SettingsUI.Container>
            )}
        </div>
    );
};
