import { invoke } from '@tauri-apps/api/core';
import { DatabaseZap } from 'lucide-react';
import { toast } from 'react-toastify';
import { Page } from '@/components/page';
import { useTranslation } from '@/i18n';
import { WizardStep } from '../wizard-step/wizard-step';
import { StepProps } from './step.types';

// Plumed fork (written with Claude Code): asks once whether Plumed may look
// for medical data updates at each start. No check, so no network call, is
// made before the answer; a "yes" checks from the next start on.
export const MedicalDataStep = ({ onNext }: StepProps) => {
    const { t } = useTranslation();

    const answer = async (enabled: boolean) => {
        try {
            await invoke('set_medical_data_update_check', { enabled });
        } catch (e) {
            toast.error(t('Failed to save the medical data update setting'));
            console.error(e);
            return;
        }
        onNext();
    };

    return (
        <WizardStep
            icon={DatabaseZap}
            title={t('Check for medical rules updates?')}
            description={t(
                'At each start, Plumed can check on GitHub whether newer medical correction lists are available: model correction rules, sound-alike word groups and medical vocabulary.'
            )}
            footer={
                <>
                    <Page.SecondaryButton onClick={() => void answer(false)} data-testid="medical-data-question-no">
                        {t("Don't check")}
                    </Page.SecondaryButton>
                    <Page.PrimaryButton onClick={() => void answer(true)} data-testid="medical-data-question-yes">
                        {t('Check at each start')}
                    </Page.PrimaryButton>
                </>
            }
        >
            <ul className="list-disc pl-5 text-sm space-y-1">
                <li>{t('Nothing is downloaded without asking you first.')}</li>
                <li>{t('Nothing about you is sent: no identifier, no cookie, no usage data.')}</li>
                <li>{t('You can change this choice in Settings > System > Online.')}</li>
            </ul>
        </WizardStep>
    );
};
