import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { MicOff, TriangleAlert } from 'lucide-react';
import { toast } from 'react-toastify';
import { Page } from '@/components/page';
import { useTranslation } from '@/i18n';
import { RemoteMicRelayDialog } from '@/features/extensions/remote-mic/remote-mic-relay-dialog/remote-mic-relay-dialog';
import { WizardStep } from '../wizard-step/wizard-step';
import { StepProps } from './step.types';

// Plumed fork (written with Claude Code): when no microphone is found, offers
// the phone as microphone (the backend decides when it is due).
export const RemoteMicStep = ({ onNext }: StepProps) => {
    const { t } = useTranslation();
    const [choosingRelay, setChoosingRelay] = useState(false);

    const answer = async (enable: boolean) => {
        setChoosingRelay(false);
        try {
            await invoke('answer_remote_mic_prompt', { enable });
        } catch (e) {
            toast.error(t('Failed to save the PhoneMic setting'));
            console.error(e);
            return;
        }
        onNext();
    };

    return (
        <>
            <WizardStep
                icon={MicOff}
                title={t('No microphone found')}
                description={t(
                    'Plumed did not find a microphone on this computer. Do you want to use your phone as a microphone instead, with PhoneMic?'
                )}
                footer={
                    <>
                        <Page.SecondaryButton onClick={() => void answer(false)} data-testid="remote-mic-prompt-no">
                            {t('No thanks')}
                        </Page.SecondaryButton>
                        <Page.PrimaryButton onClick={() => setChoosingRelay(true)} data-testid="remote-mic-prompt-yes">
                            {t('Use my phone')}
                        </Page.PrimaryButton>
                    </>
                }
            >
                <div className="flex gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm text-amber-500">
                    <TriangleAlert className="w-4 h-4 shrink-0 mt-0.5" />
                    <span>{t('PhoneMic is an alpha feature: avoid sending sensitive content through it.')}</span>
                </div>
                <p className="text-sm">
                    {t(
                        'Any cheap microphone works well with Plumed: a USB or headset microphone costs less than 5 euros online.'
                    )}
                </p>
            </WizardStep>
            <RemoteMicRelayDialog
                open={choosingRelay}
                onCancel={() => void answer(false)}
                onConfirmed={() => void answer(true)}
            />
        </>
    );
};
