import { useEffect } from 'react';
import { NumberInput } from '@/components/number-input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/select';
import { SettingsUI } from '@/components/settings-ui';
import { Slider } from '@/components/slider';
import { Switch } from '@/components/switch';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { useFormattingRules } from '@/features/personalize/formatting-rules/hooks/use-formatting-rules';
import { useRemoveHesitations } from '@/features/personalize/formatting-rules/hooks/use-remove-hesitations';
import { PipelineStepId, RULES_ANCHORS, pipelineStepTitle, stepNumber } from '../corrections.helpers';

interface CleanUpTabProps {
    /** Id of the block to scroll to once loaded (see `RULES_ANCHORS`). */
    anchor: string | null;
}

/** "Clean-up and standardization" tab: the pre-installed options of steps 4, 6 and 8. */
export const CleanUpTab = ({ anchor }: CleanUpTabProps) => {
    const { t } = useTranslation();
    const { settings, isLoading, updateBuiltInOption } = useFormattingRules();
    const { removeHesitations, setRemoveHesitations } = useRemoveHesitations();
    const shortTextThreshold = settings.built_in.short_text_correction;

    useEffect(() => {
        if (anchor == null || isLoading) {
            return;
        }
        document.getElementById(anchor)?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }, [anchor, isLoading]);

    if (isLoading) {
        return <div className="text-muted-foreground">{t('Loading...')}</div>;
    }

    const stepHeading = (id: PipelineStepId) => `(${stepNumber(id)}) ${pipelineStepTitle(id, t)}`;

    return (
        <div className="space-y-6" data-testid="built-in-options">
            <Typography.Paragraph>
                {t(
                    'The pre-installed options that clean up and standardize the text. They run at steps 4, 6 and 8 of the Explanation tab.'
                )}
            </Typography.Paragraph>
            <section className="space-y-3">
                <Typography.Title>{stepHeading('clean_up')}</Typography.Title>
                <SettingsUI.Container id={RULES_ANCHORS.hesitations}>
                    <SettingsUI.Item>
                        <SettingsUI.Description className="w-150">
                            <Typography.Title>{t('Remove hesitation sounds')}</Typography.Title>
                            <Typography.Paragraph>
                                {t('Removes sounds like "euh", "um" and "hmm" from transcriptions.')}
                                <br />
                                <span className="text-xs italic text-muted-foreground">
                                    {t('Example: "I um think so" → "I think so"')}
                                </span>
                            </Typography.Paragraph>
                        </SettingsUI.Description>
                        <Switch
                            checked={removeHesitations}
                            onCheckedChange={setRemoveHesitations}
                            data-testid="option-remove-hesitations"
                        />
                    </SettingsUI.Item>
                </SettingsUI.Container>
            </section>

            <section className="space-y-3">
                <Typography.Title>{stepHeading('short_text')}</Typography.Title>
                <SettingsUI.Container id={RULES_ANCHORS.shortText}>
                    <SettingsUI.Item>
                        <SettingsUI.Description className="w-150">
                            <Typography.Title>{t('Short text correction')}</Typography.Title>
                            <Typography.Paragraph>
                                {shortTextThreshold > 0 ? (
                                    <>
                                        {t(
                                            'Removes capitalization and punctuation for transcriptions of {{count}} word(s) or fewer.',
                                            { count: shortTextThreshold }
                                        )}
                                        <br />
                                        <span className="text-xs italic text-muted-foreground">
                                            {t('Example: "Hello." → "hello"')}
                                        </span>
                                    </>
                                ) : (
                                    <span className="italic text-muted-foreground">
                                        {t('Disabled. Move the slider to activate.')}
                                    </span>
                                )}
                            </Typography.Paragraph>
                        </SettingsUI.Description>
                        <Slider
                            value={[shortTextThreshold]}
                            onValueChange={([value]) => updateBuiltInOption('short_text_correction', value)}
                            min={0}
                            max={5}
                            step={1}
                            showValue
                            className="w-28"
                            data-testid="option-short-text-correction"
                        />
                    </SettingsUI.Item>
                </SettingsUI.Container>
            </section>

            <section className="space-y-3">
                <Typography.Title>{stepHeading('built_in_formatting')}</Typography.Title>
                <SettingsUI.Container id={RULES_ANCHORS.formattingOptions}>
                    <SettingsUI.Item>
                        <SettingsUI.Description className="w-150">
                            <Typography.Title>{t('Add space before ? and !')}</Typography.Title>
                            <Typography.Paragraph>
                                {t(
                                    'Automatically adds a space before question marks and exclamation points if missing.'
                                )}
                                <br />
                                <span className="text-xs italic text-muted-foreground">
                                    {t('Example: "Hello?" → "Hello ?"')}
                                </span>
                            </Typography.Paragraph>
                        </SettingsUI.Description>
                        <Switch
                            checked={settings.built_in.space_before_punctuation}
                            onCheckedChange={(checked) => updateBuiltInOption('space_before_punctuation', checked)}
                            data-testid="option-space-before-punctuation"
                        />
                    </SettingsUI.Item>
                </SettingsUI.Container>

                <SettingsUI.Container>
                    <SettingsUI.Item>
                        <SettingsUI.Description className="w-150">
                            <Typography.Title>{t('Add space at end of transcription')}</Typography.Title>
                            <Typography.Paragraph>
                                {t(
                                    'Ensures each transcription ends with a space. Prevents consecutive transcriptions from "sticking" together.'
                                )}
                            </Typography.Paragraph>
                        </SettingsUI.Description>
                        <Switch
                            checked={settings.built_in.trailing_space}
                            onCheckedChange={(checked) => updateBuiltInOption('trailing_space', checked)}
                            data-testid="option-trailing-space"
                        />
                    </SettingsUI.Item>
                </SettingsUI.Container>

                <SettingsUI.Container>
                    <SettingsUI.Item>
                        <SettingsUI.Description className="w-150">
                            <Typography.Title>{t('Convert text numbers to digits')}</Typography.Title>
                            <Typography.Paragraph>
                                {t('Automatically converts numbers written in letters to digits.')}
                                <br />
                                <span className="text-xs italic text-muted-foreground">
                                    {t('Example: "one" → "1", "twenty-three" → "23"')}
                                </span>
                            </Typography.Paragraph>
                        </SettingsUI.Description>
                        <Switch
                            checked={settings.built_in.convert_text_numbers}
                            onCheckedChange={(checked) => updateBuiltInOption('convert_text_numbers', checked)}
                            data-testid="option-convert-text-numbers"
                        />
                    </SettingsUI.Item>
                    {settings.built_in.convert_text_numbers && (
                        <>
                            <SettingsUI.Separator />
                            <SettingsUI.Item>
                                <SettingsUI.Description className="flex-1">
                                    <Typography.Title>{t('Language for number conversion')}</Typography.Title>
                                    <Typography.Paragraph>
                                        {t('Choose the language for text-to-number conversion')}
                                    </Typography.Paragraph>
                                </SettingsUI.Description>
                                <Select
                                    value={settings.built_in.text_numbers_language}
                                    onValueChange={(value) => updateBuiltInOption('text_numbers_language', value)}
                                >
                                    <SelectTrigger className="w-45">
                                        <SelectValue placeholder={t('Select language')} />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="en">English</SelectItem>
                                        <SelectItem value="fr">Français</SelectItem>
                                        <SelectItem value="de">Deutsch</SelectItem>
                                        <SelectItem value="it">Italiano</SelectItem>
                                        <SelectItem value="es">Español</SelectItem>
                                        <SelectItem value="nl">Nederlands</SelectItem>
                                        <SelectItem value="pt">Português</SelectItem>
                                    </SelectContent>
                                </Select>
                            </SettingsUI.Item>
                            <SettingsUI.Separator />
                            <SettingsUI.Item>
                                <SettingsUI.Description className="w-150">
                                    <Typography.Title>{t('Conversion threshold')}</Typography.Title>
                                    <Typography.Paragraph>
                                        {t('Do not convert numbers that are strictly below this threshold.')}
                                    </Typography.Paragraph>
                                </SettingsUI.Description>
                                <div className="w-30">
                                    <NumberInput
                                        value={settings.built_in.text_numbers_threshold}
                                        onValueChange={(value) =>
                                            updateBuiltInOption('text_numbers_threshold', value ?? 10)
                                        }
                                        min={0}
                                        max={50}
                                    />
                                </div>
                            </SettingsUI.Item>
                        </>
                    )}
                </SettingsUI.Container>
            </section>
        </div>
    );
};
