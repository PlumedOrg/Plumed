import { Download } from 'lucide-react';
import { Page } from '@/components/page';
import { useTranslation } from '@/i18n';
import { useModelStatus } from '@/features/model/hooks/use-model-status';
import { ModelChooser } from '@/features/model/model-chooser/model-chooser';
import { WizardStep } from '../wizard-step/wizard-step';
import { StepProps } from './step.types';

// Plumed fork (written with Claude Code): the speech model is not bundled.
// A download runs in the backend, so it keeps going while the next steps are
// answered.
export const ModelStep = ({ onNext }: StepProps) => {
    const { t } = useTranslation();
    const { status, setStatus } = useModelStatus();

    return (
        <WizardStep
            icon={Download}
            title={t('Choose a speech model')}
            description={t(
                'Plumed needs a speech model to transcribe. It runs on this computer: your recordings never leave it.'
            )}
            footer={
                <Page.PrimaryButton onClick={onNext} data-testid="wizard-model-next">
                    {t('Continue')}
                </Page.PrimaryButton>
            }
        >
            {status !== null && <ModelChooser status={status} onStatus={setStatus} />}
            <p className="text-xs text-muted-foreground">
                {t(
                    'The download happens once and is kept across updates. It continues in the background while you answer the next questions. You can change the model later in Settings > System > Model.'
                )}
            </p>
        </WizardStep>
    );
};
