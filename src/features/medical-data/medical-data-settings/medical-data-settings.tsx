import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Loader2 } from 'lucide-react';
import { toast } from 'react-toastify';
import { Button } from '@/components/button';
import { SettingsUI } from '@/components/settings-ui';
import { Switch } from '@/components/switch';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { useBooleanSetting } from '@/features/settings/hooks/use-boolean-setting';
import { useMedicalDataChangeToast } from '../hooks/use-medical-data-change-toast';
import { MedicalDataUpdateDialog } from '../medical-data-update-dialog/medical-data-update-dialog';
import { useMedicalDataStatus } from '../hooks/use-medical-data-status';
import { medicalDataVersionLabel } from '../medical-data.helpers';
import { MedicalDataChange, MedicalDataOffer } from '../types';

// Plumed fork (written with Claude Code): which medical data is in use, the
// startup check toggle, "Check now" and "Return to the bundled data".
// Hidden in builds without the signing public key.
export const MedicalDataSettings = () => {
    const { t } = useTranslation();
    const showChange = useMedicalDataChangeToast();
    const { status, refresh } = useMedicalDataStatus();
    const [offer, setOffer] = useState<MedicalDataOffer | null>(null);
    const [busy, setBusy] = useState(false);
    const [checkEnabled, setCheckEnabled] = useBooleanSetting(
        'medical_data_update_check',
        'set_medical_data_update_check',
        t('Failed to save the medical data update setting')
    );

    if (status == null || !status.available) {
        return null;
    }

    const checkNow = async () => {
        setBusy(true);
        try {
            const found = await invoke<MedicalDataOffer | null>('check_medical_data_update');
            if (found == null) {
                toast.info(t('The medical data is up to date.'));
            }
            setOffer(found);
        } catch (e) {
            toast.error(`${t('Failed to check for a medical data update')}: ${String(e)}`);
        } finally {
            setBusy(false);
        }
    };

    const returnToBundled = async () => {
        setBusy(true);
        try {
            showChange(await invoke<MedicalDataChange>('reset_medical_data'));
            refresh();
        } catch (e) {
            toast.error(`${t('Failed to return to the bundled medical data')}: ${String(e)}`);
        } finally {
            setBusy(false);
        }
    };

    return (
        <SettingsUI.Container>
            <SettingsUI.Item>
                <SettingsUI.Description className="w-150">
                    <Typography.Title>{t('Medical data updates')}</Typography.Title>
                    <Typography.Paragraph data-testid="medical-data-version">
                        {medicalDataVersionLabel(status, t)}
                    </Typography.Paragraph>
                    <Typography.Paragraph className="text-xs">
                        {t(
                            'At startup, asks GitHub whether newer signed rules exist for this model. Nothing is downloaded without your click, and your own edits are kept.'
                        )}
                    </Typography.Paragraph>
                </SettingsUI.Description>
                <Switch
                    checked={checkEnabled}
                    onCheckedChange={setCheckEnabled}
                    aria-label={t('Check for medical data updates at startup')}
                    data-testid="medical-data-update-check"
                />
            </SettingsUI.Item>
            <div className="flex items-center gap-2 px-4 pb-4">
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
            <MedicalDataUpdateDialog offer={offer} onClose={() => setOffer(null)} onInstalled={refresh} />
        </SettingsUI.Container>
    );
};
