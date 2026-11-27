import { ShieldOff } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { BooleanSettingItem } from '../boolean-setting-item/boolean-setting-item';

export const BlockCliImportSettings = () => {
    const { t } = useTranslation();
    return (
        <BooleanSettingItem
            settingKey="block_cli_import"
            command="set_block_cli_import"
            errorMessage={t('Failed to save the command-line import setting')}
            icon={ShieldOff}
            title={t('Block command-line import')}
            description={t(
                'Refuse "plumed import" from the command line, so another program cannot silently replace your settings. Importing from this window still works.'
            )}
        />
    );
};
