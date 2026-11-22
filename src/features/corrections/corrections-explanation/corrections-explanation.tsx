import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { PreviewArrow } from '@/features/extensions/extension-demo/preview-arrow/preview-arrow';
import { MedicalDataSettings } from '@/features/medical-data/medical-data-settings/medical-data-settings';
import { useMedicalDataStatus } from '@/features/medical-data/hooks/use-medical-data-status';
import { medicalDataVersionLabel } from '@/features/medical-data/medical-data.helpers';
import { useFormattingRules } from '@/features/personalize/formatting-rules/hooks/use-formatting-rules';
import { useRemoveHesitations } from '@/features/personalize/formatting-rules/hooks/use-remove-hesitations';
import { useMedicalRules } from '@/features/personalize/medical-rules/hooks/use-medical-rules';
import { CorrectionsTab, CorrectionsTarget, RULES_ANCHORS } from '../corrections.helpers';
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

    useEffect(() => {
        invoke<string[]>('get_dictionary')
            .then((words) => setDictionaryCount(words.length))
            .catch((error: unknown) => console.error('Failed to load the personal dictionary:', error));
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
    const openOption = (anchor: string, testId: string): StepAction => ({
        label: t('Open the option'),
        onClick: () => onOpen({ tab: 'formatting', anchor, builtIn: true }),
        testId,
    });
    const enabledRules = settings.rules.filter((rule) => rule.enabled).length;
    const onText = t('On the text');

    const steps: Step[] = [
        {
            title: t('Boost medical vocabulary'),
            stage: t('During recognition'),
            description: t(
                'While it recognizes your voice, the model is nudged towards the words and phrases of a medical list of about 77,000 entries: once "microangiopathie" is recognized, "thrombotique" becomes more likely.'
            ),
            examples: [{ before: 'microangiopathie trombotique', after: 'microangiopathie thrombotique' }],
            statuses: [onOff(options.phrase_boosting)],
            actions: [openTab('vocabulary', t('Open the medical vocabulary'), 'corrections-open-vocabulary')],
        },
        {
            title: t('Tell apart words that sound alike'),
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
        {
            title: t('Restore the spelling of the personal dictionary'),
            stage: onText,
            description: t(
                'A recognized word very close to a word of your personal dictionary takes its spelling (accents, capitals). These words are also favored during recognition, then their spelling is restored here.'
            ),
            examples: [{ before: 'maladie de dupuytrin', after: 'maladie de Dupuytren' }],
            statuses: [
                {
                    label: t('{{count}} word(s) of the personal dictionary', { count: dictionaryCount }),
                    active: dictionaryCount > 0,
                },
            ],
            actions: [openTab('dictionary', t('Open the personal dictionary'), 'corrections-open-dictionary')],
        },
        {
            title: t('Clean up hesitations'),
            stage: onText,
            description: t(
                'Words repeated by mistake are removed, and hesitation sounds like "euh" too when the option is on.'
            ),
            examples: [
                { before: 'le le patient', after: 'le patient' },
                { before: 'veine euh lafaxine', after: 'veine lafaxine' },
            ],
            statuses: [onOff(removeHesitations)],
            actions: [openOption(RULES_ANCHORS.hesitations, 'corrections-open-hesitations')],
        },
        {
            title: t('Fix frequent model errors'),
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
                openTab('model-corrections', t('Open the model corrections'), 'corrections-open-model-corrections'),
            ],
        },
        {
            title: t('Short text correction'),
            stage: onText,
            description:
                options.short_text_correction > 0
                    ? t('Removes capitalization and punctuation for transcriptions of {{count}} word(s) or fewer.', {
                          count: options.short_text_correction,
                      })
                    : t('Removes capitalization and punctuation for short transcriptions.'),
            examples: [{ before: 'Oui.', after: 'oui' }],
            statuses: [onOff(options.short_text_correction > 0)],
            actions: [openOption(RULES_ANCHORS.shortText, 'corrections-open-short-text')],
        },
        {
            title: t('Your formatting rules'),
            stage: onText,
            description: t(
                'Your own find and replace rules, applied from top to bottom. A fresh install starts with the medical dictation rules of InterHop.'
            ),
            examples: [{ before: 'tension 120 80', after: 'TA : 120/80' }],
            statuses: [{ label: t('{{count}} rule(s) on', { count: enabledRules }), active: enabledRules > 0 }],
            actions: [openTab('formatting', t('Open the formatting rules'), 'corrections-open-user-rules')],
        },
        {
            title: t('Pre-installed formatting'),
            stage: onText,
            description: t(
                'Last, the pre-installed options: space before ? and !, numbers written as digits, and a space at the end of each transcription.'
            ),
            examples: [{ before: 'Des douleurs?', after: 'Des douleurs ?' }],
            statuses: [
                onOff(options.space_before_punctuation || options.convert_text_numbers || options.trailing_space),
            ],
            actions: [openOption(RULES_ANCHORS.formattingOptions, 'corrections-open-formatting-options')],
        },
    ];

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

            {dataStatus != null && (
                <SettingsUI.Container className="space-y-2 p-4" data-testid="corrections-medical-data">
                    <Typography.Title>{t('Medical data')}</Typography.Title>
                    <Typography.Paragraph>
                        {t(
                            'The medical vocabulary (step 1), the sound-alike words (step 2) and the model corrections (step 5) form the medical data.'
                        )}{' '}
                        {medicalDataVersionLabel(dataStatus, t)}
                    </Typography.Paragraph>
                    {dataStatus.available && (
                        <Typography.Paragraph>
                            {t(
                                'When a signed update is offered online, Plumed asks before downloading it. Your own changes are kept.'
                            )}
                        </Typography.Paragraph>
                    )}
                </SettingsUI.Container>
            )}
            <MedicalDataSettings />
        </div>
    );
};
