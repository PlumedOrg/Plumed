import { Loader2, RefreshCcw } from 'lucide-react';
import { Button } from '@/components/button';
import { ExternalLink } from '@/components/external-link';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { AppUpdateChannelSelect } from '@/features/update-checker/app-update-channel-select/app-update-channel-select';
import { useCheckAppUpdateNow } from '@/features/update-checker/hooks/use-check-app-update-now';
import { PLUMED_RELEASES_URL } from '@/utils/plumed-links';

// Plumed fork (written with Claude Code): which app releases the updater
// offers (see AGENTS.FORK.md, auto-updater section), and a check on demand.
export const AppUpdateSettings = () => {
    const { t } = useTranslation();
    const { isChecking, checkNow } = useCheckAppUpdateNow();

    return (
        <>
            <SettingsUI.Item>
                <SettingsUI.Description>
                    <Typography.Title className="flex items-center gap-2">
                        <RefreshCcw className="w-4 h-4 text-muted-foreground" />
                        {t('App updates')}
                    </Typography.Title>
                    <Typography.Paragraph>
                        {t(
                            'Plumed checks for a new version at each start, then once a day if it stays open. Beta versions get new features first and may have more bugs. When disabled, Plumed never checks by itself; a button in the sidebar checks on demand.'
                        )}
                    </Typography.Paragraph>
                </SettingsUI.Description>
                <AppUpdateChannelSelect />
            </SettingsUI.Item>
            <div className="flex items-center gap-4 px-4 pb-4">
                <Button
                    variant="secondary"
                    size="sm"
                    disabled={isChecking}
                    onClick={() => void checkNow()}
                    data-testid="app-update-check-now"
                >
                    {isChecking && <Loader2 className="h-4 w-4 animate-spin" />}
                    {t('Check now')}
                </Button>
                <ExternalLink href={PLUMED_RELEASES_URL}>{t('Open the releases page')}</ExternalLink>
            </div>
        </>
    );
};
