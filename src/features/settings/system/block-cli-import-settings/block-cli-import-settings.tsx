import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { Switch } from '@/components/switch';
import { ShieldOff } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { useBlockCliImportState } from './hooks/use-block-cli-import-state';

export const BlockCliImportSettings = () => {
    const { blockCliImport, setBlockCliImport } = useBlockCliImportState();
    const { t } = useTranslation();

    return (
        <SettingsUI.Item>
            <SettingsUI.Description>
                <Typography.Title className="flex items-center gap-2">
                    <ShieldOff className="w-4 h-4 text-muted-foreground" />
                    {t('Block command-line import')}
                </Typography.Title>
                <Typography.Paragraph>
                    {t(
                        'Refuse "plumed import" from the command line, so another program cannot silently replace your settings. Importing from this window still works.'
                    )}
                </Typography.Paragraph>
            </SettingsUI.Description>
            <Switch checked={blockCliImport} onCheckedChange={setBlockCliImport} />
        </SettingsUI.Item>
    );
};
