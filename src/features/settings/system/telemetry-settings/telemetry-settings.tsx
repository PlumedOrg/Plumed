import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { Switch } from '@/components/switch';
import { ChartNoAxesColumn } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { useBooleanSetting } from '@/features/settings/hooks/use-boolean-setting';
import { TelemetryDetails } from '@/features/telemetry/telemetry-details/telemetry-details';

// Plumed fork (written with Claude Code): the opt-in usage statistics toggle.
// Turning it off deletes the random identifier and the counters.
export const TelemetrySettings = () => {
    const { t } = useTranslation();
    const [enabled, setEnabled] = useBooleanSetting(
        'telemetry_enabled',
        'set_telemetry_enabled',
        t('Failed to save the usage statistics setting')
    );

    return (
        <SettingsUI.Item>
            <SettingsUI.Description>
                <Typography.Title className="flex items-center gap-2">
                    <ChartNoAxesColumn className="w-4 h-4 text-muted-foreground" />
                    {t('Send anonymous statistics to the developer?')}
                </Typography.Title>
                <TelemetryDetails />
            </SettingsUI.Description>
            <Switch checked={enabled} onCheckedChange={setEnabled} data-testid="telemetry-switch" />
        </SettingsUI.Item>
    );
};
