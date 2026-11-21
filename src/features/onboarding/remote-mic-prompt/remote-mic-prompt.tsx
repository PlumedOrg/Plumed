import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useNavigate } from '@tanstack/react-router';
import { MicOff, TriangleAlert } from 'lucide-react';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/dialog';
import { Page } from '@/components/page';
import { Typography } from '@/components/typography';
import { RemoteMicRelayDialog } from '@/features/extensions/remote-mic/remote-mic-relay-dialog/remote-mic-relay-dialog';

// Plumed fork: at startup and after each update, when no microphone is found,
// offers the phone as microphone (the backend decides when it is due).
export const RemoteMicPrompt = () => {
    const { t } = useTranslation();
    const navigate = useNavigate();
    const [open, setOpen] = useState(false);
    const [choosingRelay, setChoosingRelay] = useState(false);

    useEffect(() => {
        invoke<boolean>('remote_mic_prompt_due')
            .then(setOpen)
            .catch((e) => console.error('Failed to check the remote microphone prompt:', e));
    }, []);

    const answer = async (enable: boolean) => {
        setOpen(false);
        setChoosingRelay(false);
        try {
            await invoke('answer_remote_mic_prompt', { enable });
        } catch (e) {
            toast.error(t('Failed to save the PhoneMic setting'));
            console.error(e);
            return;
        }
        if (enable) {
            void navigate({ to: '/extensions/remote-mic' });
        }
    };

    // Closing the dialog counts as "no": it comes back after the next update.
    const handleOpenChange = (isOpen: boolean) => {
        if (!isOpen && !choosingRelay) {
            void answer(false);
        }
    };

    return (
        <>
            <Dialog open={open} onOpenChange={handleOpenChange}>
                <DialogContent className="max-w-lg" overlayClassName="backdrop-blur-sm" data-testid="remote-mic-prompt">
                    <DialogHeader className="space-y-3">
                        <div className="flex items-center gap-3">
                            <div className="w-8 h-8 bg-sky-500/15 rounded-full flex items-center justify-center flex-shrink-0">
                                <MicOff className="w-4 h-4 text-sky-400" />
                            </div>
                            <DialogTitle>{t('No microphone found')}</DialogTitle>
                        </div>
                        <DialogDescription>
                            {t(
                                'Plumed did not find a microphone on this computer. Do you want to use your phone as a microphone instead, with PhoneMic?'
                            )}
                        </DialogDescription>
                    </DialogHeader>

                    <div className="flex gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm text-amber-500">
                        <TriangleAlert className="w-4 h-4 shrink-0 mt-0.5" />
                        <span>
                            {t('PhoneMic is an alpha feature: it may not be suitable for real patient data yet.')}
                        </span>
                    </div>
                    <Typography.Paragraph>
                        {t(
                            'Any cheap microphone works well with Plumed: a USB or headset microphone costs less than 5 euros online.'
                        )}
                    </Typography.Paragraph>

                    <DialogFooter>
                        <Page.SecondaryButton onClick={() => void answer(false)} data-testid="remote-mic-prompt-no">
                            {t('No thanks')}
                        </Page.SecondaryButton>
                        <Page.PrimaryButton
                            onClick={() => {
                                setOpen(false);
                                setChoosingRelay(true);
                            }}
                            data-testid="remote-mic-prompt-yes"
                        >
                            {t('Use my phone')}
                        </Page.PrimaryButton>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
            <RemoteMicRelayDialog
                open={choosingRelay}
                onCancel={() => void answer(false)}
                onConfirmed={() => void answer(true)}
            />
        </>
    );
};
