import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { PreviewArrow } from '@/features/extensions/extension-demo/preview-arrow/preview-arrow';
import { InternalLink } from '@/components/internal-link';
import { useMedicalDataStatus } from '@/features/medical-data/hooks/use-medical-data-status';
import { medicalDataVersionLabel } from '@/features/medical-data/medical-data.helpers';
import { useFormattingRules } from '@/features/personalize/formatting-rules/hooks/use-formatting-rules';
import { useRemoveHesitations } from '@/features/personalize/formatting-rules/hooks/use-remove-hesitations';
import { useMedicalRules } from '@/features/personalize/medical-rules/hooks/use-medical-rules';
import {
    CorrectionsTab,
    CorrectionsTarget,
    PIPELINE_STEPS,
    PipelineStepId,
    RULES_ANCHORS,
    cleanUpOptionTiming,
    pipelineStepTitle,
    stepNumber,
} from '../corrections.helpers';
import { HistorySteps } from '@/features/home/history/history-steps/history-steps';
import { EXAMPLE_STEPS } from './example-steps';
import { PipelineStep, Step, StepAction, StepStatus } from './pipeline-step/pipeline-step';

interface CorrectionsExplanationProps {
    onOpen: (target: CorrectionsTarget) => void;
}

/**
 * Explanation tab: the steps that change the text, as cards linked by
 * arrows, in the order the code runs them (`audio/pipeline.rs`, then
 * `formatting_rules/formatter.rs`). Examples use French medical words.
 */
export const CorrectionsExplanation = ({ onOpen }: CorrectionsExplanationProps) => {
    const { t } = useTranslation();
    const { settings, isLoading } = useFormattingRules();
    const { removeHesitations } = useRemoveHesitations();
    const { edits } = useMedicalRules();
    const { status: dataStatus } = useMedicalDataStatus();
    const [dictionaryCount, setDictionaryCount] = useState(0);
    const [templateCount, setTemplateCount] = useState(0);

    useEffect(() => {
        invoke<string[]>('get_dictionary')
            .then((words) => setDictionaryCount(words.length))
            .catch((error: unknown) => console.error('Failed to load the personal dictionary:', error));
        invoke<number>('get_enabled_text_template_count')
            .then(setTemplateCount)
            .catch((error: unknown) => console.error('get_enabled_text_template_count failed:', error));
    }, []);

    if (isLoading) {
        return <div className="text-muted-foreground">{t('Loading...')}</div>;
    }

    const options = settings.built_in;
    const onOff = (on: boolean): StepStatus => ({ label: on ? t('On') : t('Off'), active: on });
    const openTab = (tab: CorrectionsTab, label: string, testId: string): StepAction => ({
        label,
        onClick: () => onOpen({ tab }),
        testId,
    });
    // Several statuses on one card need their name: "Hésitations : activé".
    const namedOnOff = (name: string, on: boolean): StepStatus => ({
        label: on ? t('{{name}}: on', { name }) : t('{{name}}: off', { name }),
        active: on,
    });
    const openOption = (anchor: string, label: string, testId: string): StepAction => ({
        label,
        onClick: () => onOpen({ tab: 'clean-up', anchor }),
        testId,
    });
    const enabledRules = settings.rules.filter((rule) => rule.enabled).length;
    const onText = t('On the text');

    const content: Record<PipelineStepId, Omit<Step, 'title'>> = {
        vocabulary: {
            stage: t('During recognition'),
            description: t(
                'While it recognizes your voice, the model is nudged towards the words and phrases of a medical list of about 77,000 entries: once "microangiopathie" is recognized, "thrombotique" becomes more likely.'
            ),
            examples: [{ before: 'microangiopathie trombotique', after: 'microangiopathie thrombotique' }],
            statuses: [onOff(options.phrase_boosting)],
            actions: [openTab('vocabulary', t('Open the medical vocabulary'), 'corrections-open-vocabulary')],
        },
        sound_alikes: {
            stage: t('Second listening of the audio'),
            description: t(
                'When the model writes a word that can stand for several medical terms, Plumed listens to that passage again, with the end of the previous word, and keeps the term the audio supports best. Final transcriptions only.'
            ),
            examples: [
                { before: 'instaure ituximab', after: 'instaure rituximab' },
                { before: 'on passe ituximab', after: 'on passe cetuximab' },
            ],
            note: t(
                'Here the [r] of "instaure" or the [s] of "passe" decides. The provided groups use the forms the model really writes; this example is an illustration.'
            ),
            statuses: [onOff(options.confusion_rescoring)],
            actions: [openTab('sound-alikes', t('Open the sound-alike words'), 'corrections-open-sound-alikes')],
        },
        dictionary: {
            stage: onText,
            description: t(
                'A recognized word very close to a word of your personal dictionary takes its spelling (accents, capitals). These words are also favored during recognition, then their spelling is restored here.'
            ),
            examples: [{ before: 'docteur Cornélysse', after: 'docteur Cornélis' }],
            statuses: [
                {
                    label: t('{{count}} word(s) of the personal dictionary', { count: dictionaryCount }),
                    active: dictionaryCount > 0,
                },
            ],
            actions: [openTab('dictionary', t('Open the personal dictionary'), 'corrections-open-dictionary')],
        },
        clean_up: {
            stage: onText,
            description: t(
                'Words repeated by mistake are removed, and hesitation sounds like "euh" too when the option is on. Two more options of this tab run later: short text correction ({{shortTextTiming}}) removes capitalization and punctuation from very short transcriptions, and the pre-installed formatting ({{formattingTiming}}) adds a space before ? and !, writes numbers as digits and ends each transcription with a space.',
                {
                    shortTextTiming: cleanUpOptionTiming('short_text', t) ?? '',
                    formattingTiming: cleanUpOptionTiming('built_in_formatting', t) ?? '',
                }
            ),
            examples: [
                { before: 'le le patient', after: 'le patient' },
                { before: "J'ai instauré euh du paracétamol", after: "J'ai instauré du paracétamol" },
                { before: 'Oui.', after: 'oui' },
                { before: 'Des douleurs?', after: 'Des douleurs ?' },
            ],
            statuses: [
                namedOnOff(t('Hesitations'), removeHesitations),
                namedOnOff(pipelineStepTitle('short_text', t), options.short_text_correction > 0),
                namedOnOff(
                    pipelineStepTitle('built_in_formatting', t),
                    options.space_before_punctuation || options.convert_text_numbers || options.trailing_space
                ),
            ],
            actions: [
                openOption(RULES_ANCHORS.hesitations, t('Open the hesitation option'), 'corrections-open-hesitations'),
                openOption(RULES_ANCHORS.shortText, t('Open short text correction'), 'corrections-open-short-text'),
                openOption(
                    RULES_ANCHORS.formattingOptions,
                    t('Open the formatting options'),
                    'corrections-open-formatting-options'
                ),
            ],
        },
        model_corrections: {
            stage: onText,
            description: t(
                "About 25,000 rules rewrite what the model often gets wrong on drug names and medical terms: UltiMed's rules, then Plumed's own, with your changes on top."
            ),
            examples: [{ before: 'veine lafaxine', after: 'venlafaxine' }],
            note: t(
                'This step comes after the clean-up: "veine euh lafaxine" first becomes "veine lafaxine", which the rule knows how to fix.'
            ),
            statuses: [
                onOff(options.medical_fixes),
                { label: t('{{count}} change(s) of yours', { count: edits.length }), active: edits.length > 0 },
            ],
            actions: [
                openTab('model-corrections', t('Open the residual errors'), 'corrections-open-model-corrections'),
            ],
        },
        formatting_rules: {
            stage: onText,
            description: t(
                'Your own find and replace rules, applied from top to bottom. A fresh install starts with the medical dictation rules of InterHop.'
            ),
            examples: [{ before: 'tension 120 80', after: 'TA : 120/80' }],
            statuses: [{ label: t('{{count}} rule(s) on', { count: enabledRules }), active: enabledRules > 0 }],
            actions: [openTab('formatting', t('Open the formatting rules'), 'corrections-open-user-rules')],
        },
        grammar: {
            stage: onText,
            description: t(
                'The Grammalecte grammar checker then reads the corrected text. Plumed applies a fix only when the rule is on and there is a single possible fix; ages, doses, acronyms and names are hidden from the checker so it does not take them for errors.'
            ),
            examples: [{ before: 'La patiente de 63 ans est fatigué.', after: 'La patiente de 63 ans est fatiguée.' }],
            note: t('Without the hiding, "ans" would be taken as the subject and "fatigué" left as it is.'),
            statuses: [onOff(options.grammar)],
            actions: [openTab('grammar', t('Open the grammar rules'), 'corrections-open-grammar')],
        },
        templates: {
            stage: onText,
            description: t(
                'Last, a spoken phrase is replaced by a longer text written in advance. It comes after the grammar, so the text is inserted exactly as you wrote it.'
            ),
            examples: [{ before: "Insère l'examen clinique normal.", after: t('A complete normal clinical exam') }],
            statuses: [
                {
                    label: t('{{count}} text template(s) on', { count: templateCount }),
                    active: templateCount > 0,
                },
            ],
            actions: [openTab('templates', t('Open the text templates'), 'corrections-open-templates')],
        },
    };
    const steps: Step[] = PIPELINE_STEPS.map(({ id }) => ({ title: pipelineStepTitle(id, t), ...content[id] }));

    return (
        <div className="space-y-6" data-testid="corrections-explanation">
            <Typography.Paragraph>
                {t(
                    'Plumed corrects the transcribed text in several steps, always in this order. Each card shows an example, whether the step is on, and a button to open its list or option.'
                )}
            </Typography.Paragraph>

            <div className="flex flex-col gap-2">
                {steps.map((step, index) => (
                    <React.Fragment key={step.title}>
                        {index > 0 && <PreviewArrow horizontalFrom="never" />}
                        <PipelineStep index={index + 1} step={step} />
                    </React.Fragment>
                ))}
            </div>

            <SettingsUI.Container className="space-y-2 p-4" data-testid="corrections-example">
                <Typography.Title>{t('Example')}</Typography.Title>
                <Typography.Paragraph>
                    {t(
                        'A dictated sentence as each step corrects it, shown like the details of a dictation on the Home page: removed in red, added in green. An illustration, not one of your dictations.'
                    )}
                </Typography.Paragraph>
                <HistorySteps steps={EXAMPLE_STEPS} />
            </SettingsUI.Container>

            {dataStatus != null && (
                <SettingsUI.Container className="space-y-2 p-4" data-testid="corrections-medical-data">
                    <Typography.Title>{t('Medical rules')}</Typography.Title>
                    <Typography.Paragraph>
                        {t(
                            'The medical vocabulary (step {{vocabulary}}), the sound-alike words (step {{soundAlikes}}) and the residual errors (step {{residualErrors}}) form the medical rules.',
                            {
                                vocabulary: stepNumber('vocabulary'),
                                soundAlikes: stepNumber('sound_alikes'),
                                residualErrors: stepNumber('model_corrections'),
                            }
                        )}{' '}
                        {medicalDataVersionLabel(dataStatus, t)}
                    </Typography.Paragraph>
                    {dataStatus.available && (
                        <Typography.Paragraph>
                            {t(
                                'When a signed update is offered online, Plumed asks before downloading it. Your own changes are kept.'
                            )}{' '}
                            <InternalLink
                                to="/settings"
                                hash="online"
                                data-testid="corrections-medical-data-settings-link"
                            >
                                {t('Manage medical rules updates in Settings > Online')}
                            </InternalLink>
                        </Typography.Paragraph>
                    )}
                </SettingsUI.Container>
            )}
        </div>
    );
};
