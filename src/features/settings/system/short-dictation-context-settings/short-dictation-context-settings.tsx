import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { Switch } from '@/components/switch';
import { AudioLines } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { useBooleanSetting } from '@/features/settings/hooks/use-boolean-setting';

export const ShortDictationContextSettings = () => {
    const { t } = useTranslation();
    const [enabled, setEnabled] = useBooleanSetting(
        'short_dictation_context',
        'set_short_dictation_context',
        t('Failed to save the short dictation setting')
    );

    return (
        <SettingsUI.Item>
            <SettingsUI.Description>
                <Typography.Title className="flex items-center gap-2">
                    <AudioLines className="w-4 h-4 text-muted-foreground" />
                    {t('Context for short dictations')}
                </Typography.Title>
                <Typography.Paragraph>
                    {t(
                        'A very short dictation (under 2 seconds or 3 words or fewer, often a corrected word) is transcribed again behind the end of the previous dictation, which improves recognition. That audio stays in memory only, never on disk, and is forgotten after 5 minutes.'
                    )}
                </Typography.Paragraph>
            </SettingsUI.Description>
            <Switch checked={enabled} onCheckedChange={setEnabled} />
        </SettingsUI.Item>
    );
};
