import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';

interface BoostDefaults {
    weight: number;
    minp: number;
}

/** A tuning value, kept as typed until it is valid and the field loses focus. */
interface TuningField {
    label: string;
    help: string;
    value: number | null;
    listDefault: number | undefined;
    isValid: (value: number) => boolean;
    onSave: (value: number | null) => void;
    testId: string;
}

const TuningInput = ({ label, help, value, listDefault, isValid, onSave, testId }: TuningField) => {
    const [text, setText] = useState(value == null ? '' : String(value));
    useEffect(() => setText(value == null ? '' : String(value)), [value]);

    const parsed = text.trim() === '' ? null : Number(text.replace(',', '.'));
    const invalid = parsed != null && !(Number.isFinite(parsed) && isValid(parsed));
    const commit = () => {
        if (!invalid && parsed !== value) {
            onSave(parsed);
        }
    };

    return (
        <SettingsUI.Item>
            <SettingsUI.Description className="w-150">
                <Typography.Title>{label}</Typography.Title>
                <Typography.Paragraph>{help}</Typography.Paragraph>
            </SettingsUI.Description>
            <Input
                className="w-28"
                inputMode="decimal"
                value={text}
                placeholder={listDefault == null ? '' : String(listDefault)}
                aria-invalid={invalid}
                onChange={(e) => setText(e.target.value)}
                onBlur={commit}
                onKeyDown={(e) => e.key === 'Enter' && commit()}
                data-testid={testId}
            />
        </SettingsUI.Item>
    );
};

interface BoostTuningProps {
    weight: number | null;
    minp: number | null;
    onChange: (key: 'phrase_boost_weight' | 'phrase_boost_minp', value: number | null) => void;
    onReset: () => void;
}

/** Bottom of the "Medical vocabulary" tab: how strongly the list nudges the model. */
export const BoostTuning = ({ weight, minp, onChange, onReset }: BoostTuningProps) => {
    const { t } = useTranslation();
    const [defaults, setDefaults] = useState<BoostDefaults>();
    useEffect(() => {
        invoke<BoostDefaults>('get_phrase_boost_defaults')
            .then(setDefaults)
            .catch((error) => console.error('Failed to load the phrase boost defaults:', error));
    }, []);

    return (
        <SettingsUI.Container data-testid="phrase-boost-tuning">
            <TuningInput
                label={t('Boost weight')}
                help={t(
                    'How strongly the phrases of the list are favoured, from above 0 to 10. Higher finds more medical words but invents more of them. Leave empty for the original value.'
                )}
                value={weight}
                listDefault={defaults?.weight}
                isValid={(value) => value > 0 && value <= 10}
                onSave={(value) => onChange('phrase_boost_weight', value)}
                testId="phrase-boost-weight"
            />
            <SettingsUI.Separator />
            <TuningInput
                label={t('Boost margin (min-p)')}
                help={t(
                    'A phrase is only favoured once the model already finds it this likely compared to its best guess, from above 0 to 1. Lower boosts more often. Leave empty for the original value.'
                )}
                value={minp}
                listDefault={defaults?.minp}
                isValid={(value) => value > 0 && value <= 1}
                onSave={(value) => onChange('phrase_boost_minp', value)}
                testId="phrase-boost-minp"
            />
            <SettingsUI.Separator />
            <SettingsUI.Item>
                <Typography.Paragraph>
                    {defaults == null
                        ? ''
                        : t('Original values: weight {{weight}}, margin {{minp}}.', {
                              weight: defaults.weight,
                              minp: defaults.minp,
                          })}
                </Typography.Paragraph>
                <Button
                    variant="secondary"
                    size="sm"
                    disabled={weight == null && minp == null}
                    onClick={onReset}
                    data-testid="phrase-boost-tuning-reset"
                >
                    {t('Restore the original values')}
                </Button>
            </SettingsUI.Item>
        </SettingsUI.Container>
    );
};
