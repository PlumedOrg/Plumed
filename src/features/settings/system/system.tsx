import { Typography } from '@/components/typography';
import { SettingsUI } from '@/components/settings-ui';
import { Page } from '@/components/page';
import { OverlaySettings } from './overlay-settings/overlay-settings';
import { StartOnBootSettings } from './start-on-boot-settings/start-on-boot-settings';
import { DockSettings } from './dock-settings/dock-settings';
import { isMac } from '@/utils/os';
import { CopyToClipboardSettings } from './copy-to-clipboard-settings/copy-to-clipboard-settings';
import { PasteMethodSettings } from './paste-method-settings/paste-method-settings';
import { AutoInsertSettings } from './auto-insert-settings/auto-insert-settings';
import { HistorySettings } from './history-settings/history-settings';
import { LanguageSettings } from './language-settings/language-settings';
import { SoundSettings } from './sound-settings/sound-settings';
import { MicSettings } from './mic-settings/mic-settings';
import { LowerOutputSettings } from './lower-output-settings/lower-output-settings';
import { useTranslation } from '@/i18n';
import { RecordModeSettings } from '@/features/settings/system/record-mode-settings/record-mode-settings.tsx';
import { LogLevelSettings } from './log-level-settings/log-level-settings';
import { KeepRecordingsSettings } from './keep-recordings-settings/keep-recordings-settings';
import { ShortDictationContextSettings } from './short-dictation-context-settings/short-dictation-context-settings';
import { BlockCliImportSettings } from './block-cli-import-settings/block-cli-import-settings';
import { OnlineSettings } from './online-settings/online-settings';
import { Globe, Keyboard, Monitor, Settings, Wrench } from 'lucide-react';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { Tabs } from '@/components/tabs';
import { pickTabFromHash } from '@/components/tabs.helpers';
import { Shortcuts } from '@/features/settings/shortcuts/shortcuts';

// Plumed fork (written with Claude Code): the sections are tabs, and the
// keyboard shortcuts are the last one. "Online" holds every setting that
// reaches the network. The URL hash selects the tab.
const SYSTEM_TABS = ['general', 'overlay', 'advanced', 'online', 'shortcuts'] as const;
type SystemTab = (typeof SYSTEM_TABS)[number];

export const System = () => {
    const { t } = useTranslation();
    const { hash } = useLocation();
    const navigate = useNavigate();
    const activeTab = pickTabFromHash(hash, SYSTEM_TABS, 'general');
    const selectTab = (tab: SystemTab) => void navigate({ to: '/settings/system', hash: tab, replace: true });
    return (
        <main>
            <div className="space-y-8">
                <Page.Header>
                    <Typography.MainTitle data-testid="system-title">{t('System')}</Typography.MainTitle>
                    <Typography.Paragraph className="text-muted-foreground">
                        {t("Adjust system preferences to control Murmure's behavior at startup and more.")}
                    </Typography.Paragraph>
                </Page.Header>

                <Tabs
                    value={activeTab}
                    onValueChange={selectTab}
                    tabs={[
                        { value: 'general', label: t('General'), icon: Settings },
                        { value: 'overlay', label: t('Overlay'), icon: Monitor },
                        { value: 'advanced', label: t('Advanced'), icon: Wrench },
                        { value: 'online', label: t('Online'), icon: Globe },
                        { value: 'shortcuts', label: t('Shortcuts'), icon: Keyboard },
                    ]}
                >
                    {activeTab === 'general' && (
                        <SettingsUI.Container>
                            <LanguageSettings />
                            <SettingsUI.Separator />
                            <MicSettings />
                            <SettingsUI.Separator />
                            <LowerOutputSettings />
                            <SettingsUI.Separator />
                            <RecordModeSettings />
                            <SettingsUI.Separator />
                            <PasteMethodSettings />
                            <SettingsUI.Separator />
                            <AutoInsertSettings />
                            <SettingsUI.Separator />
                            <StartOnBootSettings />
                            {isMac && (
                                <>
                                    <SettingsUI.Separator />
                                    <DockSettings />
                                </>
                            )}
                            <SettingsUI.Separator />
                            <HistorySettings />
                            <SettingsUI.Separator />
                            <SoundSettings />
                        </SettingsUI.Container>
                    )}
                    {activeTab === 'overlay' && (
                        <SettingsUI.Container>
                            <OverlaySettings />
                        </SettingsUI.Container>
                    )}
                    {activeTab === 'advanced' && (
                        <SettingsUI.Container>
                            <LogLevelSettings />
                            <SettingsUI.Separator />
                            <KeepRecordingsSettings />
                            <SettingsUI.Separator />
                            <ShortDictationContextSettings />
                            <SettingsUI.Separator />
                            <CopyToClipboardSettings />
                            <SettingsUI.Separator />
                            <BlockCliImportSettings />
                        </SettingsUI.Container>
                    )}
                    {activeTab === 'online' && <OnlineSettings />}
                    {activeTab === 'shortcuts' && <Shortcuts />}
                </Tabs>
            </div>
        </main>
    );
};
