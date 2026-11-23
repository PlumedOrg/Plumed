import { Globe } from 'lucide-react';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { MedicalDataSettings } from '@/features/medical-data/medical-data-settings/medical-data-settings';
import { useMedicalDataStatus } from '@/features/medical-data/hooks/use-medical-data-status';
import { useTelemetryAvailable } from '@/features/telemetry/hooks/use-telemetry-available';
import { TelemetrySettings } from '../telemetry-settings/telemetry-settings';

// Plumed fork (written with Claude Code): the "Online" tab of the System page
// gathers everything that makes Plumed reach the network, with an intro that
// says what goes online and when. Each part is listed only when this build has
// it: usage statistics need a Umami website id, medical data updates the
// signing public key (see AGENTS.FORK.md, section 6). The app update check
// (`update-checker.tsx`, Tauri updater) runs at every start and has no
// setting, so the intro only states it.
export const OnlineSettings = () => {
    const { t } = useTranslation();
    const telemetryAvailable = useTelemetryAvailable();
    const { status: medicalDataStatus } = useMedicalDataStatus();
    const medicalDataAvailable = medicalDataStatus?.available === true;

    return (
        <div className="space-y-6" data-testid="online-settings">
            <SettingsUI.Container className="space-y-3 p-4">
                <Typography.Title className="flex items-center gap-2">
                    <Globe className="w-4 h-4 text-muted-foreground" />
                    {t('What goes online')}
                </Typography.Title>
                <Typography.Paragraph>
                    {t(
                        'Plumed transcribes on your computer: your voice, your text and your lists never leave it. Only these connections exist:'
                    )}
                </Typography.Paragraph>
                <ul className="list-disc space-y-2 pl-5 text-sm text-muted-foreground">
                    <li>
                        {t(
                            'App updates: at every start, Plumed downloads a small file from GitHub (latest.json) to see whether a newer version exists. This check cannot be turned off; an update is only installed when you click it.'
                        )}
                    </li>
                    <li>
                        {t(
                            'Speech model: downloaded once, when you choose it (Model tab), from Hugging Face or else from the Plumed relay (models.plumed.fr). Like any website, they see your IP address, nothing else.'
                        )}
                    </li>
                    {medicalDataAvailable && (
                        <li>
                            {t(
                                'Medical data updates (below): at every start, Plumed asks GitHub whether newer signed medical lists exist. Off unless you agreed; nothing is downloaded without your click.'
                            )}
                        </li>
                    )}
                    {telemetryAvailable && (
                        <li>
                            {t(
                                'Anonymous usage statistics (below): aggregated counts sent once a day on days you use Plumed. Off unless you agreed.'
                            )}
                        </li>
                    )}
                    <li>
                        {t(
                            'PhoneMic, only if you turn it on, carries the audio of your phone through a relay, encrypted end to end.'
                        )}
                    </li>
                </ul>
            </SettingsUI.Container>
            {telemetryAvailable && (
                <SettingsUI.Container>
                    <TelemetrySettings />
                </SettingsUI.Container>
            )}
            <MedicalDataSettings />
        </div>
    );
};
