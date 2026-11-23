import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Download } from 'lucide-react';
import { toast } from 'react-toastify';
import { Page } from '@/components/page';
import { useTranslation } from '@/i18n';
import { useModelStatus } from '@/features/model/hooks/use-model-status';
import { ModelChooser } from '@/features/model/model-chooser/model-chooser';
import { WizardStep } from '../wizard-step/wizard-step';
import { StepProps } from './step.types';

// Plumed fork (written with Claude Code): the speech model is not bundled.
// Continue starts the download of the chosen model (no separate button). It
// runs in the backend, so it keeps going while the next steps are answered.
export const ModelStep = ({ onNext }: StepProps) => {
    const { t } = useTranslation();
    const { status, setStatus } = useModelStatus();
    const [chosen, setChosen] = useState<string | null>(null);

    const downloadAndContinue = () => {
        const variant = status?.variants.find((v) => v.id === chosen);
        if (status !== null && status.downloading == null && variant?.available && variant.installed == null) {
            invoke('download_model', { variant: variant.id }).catch((e: unknown) =>
                toast.error(`${t('The model download failed')}: ${String(e)}`)
            );
        }
        onNext();
    };

    return (
        <WizardStep
            icon={Download}
            title={t('Choose a speech model')}
            description={t(
                'Plumed needs a speech model to transcribe. It runs on this computer: your recordings never leave it.'
            )}
            footer={
                <Page.PrimaryButton onClick={downloadAndContinue} data-testid="wizard-model-next">
                    {t('Continue')}
                </Page.PrimaryButton>
            }
        >
            {status !== null && <ModelChooser status={status} onStatus={setStatus} onChosen={setChosen} hideDownload />}
            <p className="text-xs text-muted-foreground">
                {t(
                    'The download starts when you continue, happens once and is kept across updates. It continues in the background while you answer the next questions. You can change the model later in Settings > Model.'
                )}
            </p>
        </WizardStep>
    );
};
