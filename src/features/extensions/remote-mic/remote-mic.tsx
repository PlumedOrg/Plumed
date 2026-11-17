import { Lock, RadioTower } from 'lucide-react';
import { ExtensionPageLayout } from '@/features/extensions/extension-page-layout/extension-page-layout';
import { ExtensionHeader } from '@/features/extensions/extension-header/extension-header';
import { ExtensionEnableAction } from '@/features/extensions/extension-enable-action/extension-enable-action';
import { useTranslation } from '@/i18n';
import { useRemoteMic } from './hooks/use-remote-mic';
import { useRemoteMicSettings } from './hooks/use-remote-mic-settings';
import { RemoteMicSession } from './remote-mic-session/remote-mic-session';
import { RemoteMicUrlSettings } from './remote-mic-url-settings/remote-mic-url-settings';

export const RemoteMic = () => {
    const { t } = useTranslation();
    const settings = useRemoteMicSettings();
    const session = useRemoteMic();
    const title = t('Remote microphone');

    if (!settings.enabled) {
        return (
            <ExtensionPageLayout colorScheme="standard" isActive={false}>
                <ExtensionHeader
                    icon={RadioTower}
                    title={title}
                    colorScheme="standard"
                    titleTestId="remote-mic-title"
                    subtitle={t('Dictate from your phone, even on a different network, through an encrypted relay.')}
                    badge={t('Alpha')}
                />
                <ExtensionEnableAction
                    icon={RadioTower}
                    label={t('Enable the remote microphone')}
                    onEnable={() => settings.setEnabled(true)}
                    testId="remote-mic-enable"
                    colorScheme="standard"
                    hintIcon={Lock}
                    hint={t(
                        'Audio is end-to-end encrypted: the relay cannot listen to it. Nothing is sent before you connect a phone.'
                    )}
                />
            </ExtensionPageLayout>
        );
    }

    return (
        <ExtensionPageLayout colorScheme="standard" isActive>
            <ExtensionHeader
                icon={RadioTower}
                title={title}
                colorScheme="standard"
                titleTestId="remote-mic-title"
                badge={t('Alpha')}
                toggle={{
                    checked: settings.enabled,
                    onCheckedChange: settings.setEnabled,
                    ariaLabel: t('Enable the remote microphone'),
                    testId: 'remote-mic-toggle',
                }}
            />
            <RemoteMicSession
                snapshot={session.snapshot}
                start={session.start}
                stop={session.stop}
                confirm={session.confirm}
            />
            <RemoteMicUrlSettings
                url={settings.url}
                setUrl={settings.setUrl}
                saveUrl={settings.saveUrl}
                disabled={session.snapshot.status !== 'stopped'}
            />
        </ExtensionPageLayout>
    );
};
