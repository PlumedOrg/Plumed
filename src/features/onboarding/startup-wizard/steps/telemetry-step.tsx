import { invoke } from '@tauri-apps/api/core';
import { ChartNoAxesColumn } from 'lucide-react';
import { toast } from 'react-toastify';
import { Page } from '@/components/page';
import { useTranslation } from '@/i18n';
import { TelemetryDetails } from '@/features/telemetry/telemetry-details/telemetry-details';
import { WizardStep } from '../wizard-step/wizard-step';
import { StepProps } from './step.types';

interface TelemetryStepProps extends StepProps {
    /** The third ask also offers "Never ask again". */
    offerNeverAgain: boolean;
}

// Plumed fork (written with Claude Code): asks whether to share anonymous
// usage statistics. "Later" asks again after 7 days, 10 dictations and 3
// launches (see src-tauri/src/telemetry/reask.rs).
export const TelemetryStep = ({ onNext, offerNeverAgain }: TelemetryStepProps) => {
    const { t } = useTranslation();

    const run = async (command: string, args?: Record<string, unknown>) => {
        try {
            await invoke(command, args);
        } catch (e) {
            toast.error(t('Failed to save the usage statistics setting'));
            console.error(e);
            return;
        }
        onNext();
    };

    return (
        <WizardStep
            icon={ChartNoAxesColumn}
            title={t('Send anonymous statistics to the developer?')}
            footer={
                <>
                    {offerNeverAgain && (
                        <Page.SecondaryButton
                            onClick={() => void run('set_telemetry_enabled', { enabled: false })}
                            data-testid="telemetry-prompt-never"
                        >
                            {t('Never ask again')}
                        </Page.SecondaryButton>
                    )}
                    <Page.SecondaryButton
                        onClick={() => void run('postpone_telemetry_question')}
                        data-testid="telemetry-prompt-later"
                    >
                        {t('Later')}
                    </Page.SecondaryButton>
                    <Page.PrimaryButton
                        onClick={() => void run('set_telemetry_enabled', { enabled: true })}
                        data-testid="telemetry-prompt-yes"
                    >
                        {t('Yes, send them')}
                    </Page.PrimaryButton>
                </>
            }
        >
            <TelemetryDetails />
        </WizardStep>
    );
};
