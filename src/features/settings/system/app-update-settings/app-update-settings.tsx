import { RefreshCcw } from 'lucide-react';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/select';
import { useTranslation } from '@/i18n';
import { AppUpdateChannel } from '@/features/settings/settings.types';
import { useAppUpdateChannel } from '@/features/update-checker/hooks/use-app-update-channel';

const CHANNELS: { value: AppUpdateChannel; label: string }[] = [
    { value: 'stable', label: 'Stable versions' },
    { value: 'beta', label: 'Beta versions' },
    { value: 'disabled', label: 'Disabled' },
];

// Plumed fork (written with Claude Code): which app releases the updater
// offers (see AGENTS.FORK.md, auto-updater section).
export const AppUpdateSettings = () => {
    const { t } = useTranslation();
    const [channel, setChannel] = useAppUpdateChannel();

    return (
        <SettingsUI.Item>
            <SettingsUI.Description>
                <Typography.Title className="flex items-center gap-2">
                    <RefreshCcw className="w-4 h-4 text-muted-foreground" />
                    {t('App updates')}
                </Typography.Title>
                <Typography.Paragraph>
                    {t(
                        'Beta versions get new features first and may have more bugs. When disabled, Plumed never checks by itself; a button in the sidebar checks on demand.'
                    )}
                </Typography.Paragraph>
            </SettingsUI.Description>
            <Select value={channel ?? undefined} onValueChange={(value) => void setChannel(value as AppUpdateChannel)}>
                <SelectTrigger className="w-[180px]" data-testid="app-update-channel-select">
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    {CHANNELS.map((option) => (
                        <SelectItem key={option.value} value={option.value}>
                            {t(option.label)}
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
        </SettingsUI.Item>
    );
};
