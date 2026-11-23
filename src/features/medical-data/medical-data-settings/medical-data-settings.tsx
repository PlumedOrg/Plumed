import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Loader2 } from 'lucide-react';
import { toast } from 'react-toastify';
import { Button } from '@/components/button';
import { ExternalLink } from '@/components/external-link';
import { SettingsUI } from '@/components/settings-ui';
import { Switch } from '@/components/switch';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { PLUMED_MEDICAL_DATA_RELEASE_URL } from '@/utils/plumed-links';
import { useInstallDownloadedFiles } from '../hooks/use-install-downloaded-files';
import { useMedicalDataChangeToast } from '../hooks/use-medical-data-change-toast';
import { MedicalDataUpdateDialog } from '../medical-data-update-dialog/medical-data-update-dialog';
import { useMedicalDataStatus } from '../hooks/use-medical-data-status';
import { medicalDataLastCheckLabel, medicalDataVersionLabel } from '../medical-data.helpers';
import { MedicalDataChange, MedicalDataOffer } from '../types';

// Plumed fork (written with Claude Code): which medical rules is in use, the
// startup check toggle with the last check result, "Check now", "Install a
// downloaded update" and "Return to the bundled data". Hidden in builds
// without the signing public key.
export const MedicalDataSettings = () => {
    const { t, i18n } = useTranslation();
    const showChange = useMedicalDataChangeToast();
    const { status, refresh } = useMedicalDataStatus();
    const installDownloaded = useInstallDownloadedFiles(refresh);
    const [offer, setOffer] = useState<MedicalDataOffer | null>(null);
    const [busy, setBusy] = useState(false);

    if (status == null || !status.available) {
        return null;
    }

    const whileBusy = async (job: () => Promise<void>) => {
        setBusy(true);
        try {
            await job();
        } finally {
            setBusy(false);
        }
    };

    // Also counts as the answer to the first launch question.
    const setCheckEnabled = async (enabled: boolean) => {
        try {
            await invoke('set_medical_data_update_check', { enabled });
        } catch (e) {
            toast.error(t('Failed to save the medical rules update setting'));
            console.error(e);
        }
        refresh();
    };

    const checkNow = () =>
        whileBusy(async () => {
            try {
                const found = await invoke<MedicalDataOffer | null>('check_medical_data_update');
                if (found == null) {
                    toast.info(t('The medical rules is up to date.'));
                }
                setOffer(found);
            } catch (e) {
                toast.error(`${t('Failed to check for a medical rules update')}: ${String(e)}`);
            }
            refresh();
        });

    const returnToBundled = () =>
        whileBusy(async () => {
            try {
                showChange(await invoke<MedicalDataChange>('reset_medical_data'));
                refresh();
            } catch (e) {
                toast.error(`${t('Failed to return to the bundled medical rules')}: ${String(e)}`);
            }
        });

    return (
        <SettingsUI.Container>
            <SettingsUI.Item>
                <SettingsUI.Description className="w-150">
                    <Typography.Title>{t('Medical rules updates')}</Typography.Title>
                    <Typography.Paragraph data-testid="medical-data-version">
                        {medicalDataVersionLabel(status, t)}
                    </Typography.Paragraph>
                    <Typography.Paragraph className="text-xs">
                        {t(
                            'At startup, asks GitHub whether newer signed rules exist for this model. Nothing is downloaded without your click, and your own edits are kept.'
                        )}
                    </Typography.Paragraph>
                    <Typography.Paragraph className="text-xs" data-testid="medical-data-last-check">
                        {medicalDataLastCheckLabel(status.last_check, t, i18n.language)}
                    </Typography.Paragraph>
                </SettingsUI.Description>
                <Switch
                    checked={status.check_enabled}
                    onCheckedChange={(enabled) => void setCheckEnabled(enabled)}
                    aria-label={t('Check for medical rules updates at startup')}
                    data-testid="medical-data-update-check"
                />
            </SettingsUI.Item>
            <div className="flex items-center gap-2 px-4 pb-2">
                <Button
                    variant="secondary"
                    size="sm"
                    disabled={busy}
                    onClick={() => void checkNow()}
                    data-testid="medical-data-check-now"
                >
                    {busy && <Loader2 className="h-4 w-4 animate-spin" />}
                    {t('Check now')}
                </Button>
                <Button
                    variant="secondary"
                    size="sm"
                    disabled={busy}
                    onClick={() => void whileBusy(installDownloaded)}
                    data-testid="medical-data-install-downloaded"
                >
                    {t('Install a downloaded update')}
                </Button>
                {status.downloaded_version != null && (
                    <Button
                        variant="ghost"
                        size="sm"
                        disabled={busy}
                        onClick={() => void returnToBundled()}
                        data-testid="medical-data-reset"
                    >
                        {t('Return to the bundled data')}
                    </Button>
                )}
            </div>
            <Typography.Paragraph className="text-xs px-4 pb-4">
                {t(
                    'If Plumed cannot reach GitHub, download every file of the release page with your browser, then pick them all with "Install a downloaded update". They are checked exactly like an online update.'
                )}{' '}
                <ExternalLink href={PLUMED_MEDICAL_DATA_RELEASE_URL}>{t('Open the medical rules release page')}</ExternalLink>
            </Typography.Paragraph>
            <MedicalDataUpdateDialog offer={offer} onClose={() => setOffer(null)} onInstalled={refresh} />
        </SettingsUI.Container>
    );
};
