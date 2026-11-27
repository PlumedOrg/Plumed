import { RefreshCcw } from 'lucide-react';
import { Page } from '@/components/page';
import { useTranslation } from '@/i18n';
import { AppUpdateChannelSelect } from '@/features/update-checker/app-update-channel-select/app-update-channel-select';
import { useAppUpdateChannel } from '@/features/update-checker/hooks/use-app-update-channel';
import { WizardStep } from '../wizard-step/wizard-step';
import { StepProps } from './step.types';

// Plumed fork (written with Claude Code): which app releases the updater
// offers. Confirming saves the shown channel, which also marks the question as
// answered (`app_update_channel_asked`).
export const UpdateChannelStep = ({ onNext }: StepProps) => {
    const { t } = useTranslation();
    const [channel, saveChannel] = useAppUpdateChannel();

    const confirm = async () => {
        if (channel !== null) {
            await saveChannel(channel);
        }
        onNext();
    };

    return (
        <WizardStep
            icon={RefreshCcw}
            title={t('App updates')}
            description={t(
                'Plumed checks for a new version at each start, then once a day if it stays open. Beta versions get new features first and may have more bugs. When disabled, Plumed never checks by itself; a button in the sidebar checks on demand.'
            )}
            footer={
                <Page.PrimaryButton onClick={() => void confirm()} data-testid="wizard-update-channel-next">
                    {t('Finish')}
                </Page.PrimaryButton>
            }
        >
            <AppUpdateChannelSelect />
        </WizardStep>
    );
};
