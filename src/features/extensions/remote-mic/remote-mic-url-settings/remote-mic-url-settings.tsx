import { Input } from '@/components/input';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { Server } from 'lucide-react';
import { useTranslation } from '@/i18n';

interface RemoteMicUrlSettingsProps {
    url: string;
    setUrl: (url: string) => void;
    saveUrl: () => void;
    disabled: boolean;
}

export const RemoteMicUrlSettings = ({ url, setUrl, saveUrl, disabled }: RemoteMicUrlSettingsProps) => {
    const { t } = useTranslation();
    return (
        <SettingsUI.Section title={t('Relay server')} icon={Server}>
            <SettingsUI.Item>
                <SettingsUI.Description>
                    <Typography.Title>{t('Relay URL')}</Typography.Title>
                    <Typography.Paragraph>
                        {t(
                            'Server that forwards the encrypted audio between the phone and this computer. Use only a server you or your institution trust.'
                        )}
                    </Typography.Paragraph>
                </SettingsUI.Description>
                <Input
                    value={url}
                    onChange={(e) => setUrl(e.target.value)}
                    onBlur={saveUrl}
                    placeholder="https://turn.plumed.fr"
                    className="w-72"
                    disabled={disabled}
                />
            </SettingsUI.Item>
        </SettingsUI.Section>
    );
};
