import { RefreshCcw } from 'lucide-react';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { AppUpdateChannelSelect } from '@/features/update-checker/app-update-channel-select/app-update-channel-select';

// Plumed fork (written with Claude Code): which app releases the updater
// offers (see AGENTS.FORK.md, auto-updater section).
export const AppUpdateSettings = () => {
    const { t } = useTranslation();

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
            <AppUpdateChannelSelect />
        </SettingsUI.Item>
    );
};
