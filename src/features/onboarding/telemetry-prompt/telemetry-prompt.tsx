import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ChartNoAxesColumn } from 'lucide-react';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/dialog';
import { Page } from '@/components/page';
import { AppSettings } from '@/features/settings/settings.types';
import { TelemetryDetails } from '@/features/telemetry/telemetry-details/telemetry-details';
import { useTelemetryAvailable } from '@/features/telemetry/hooks/use-telemetry-available';
import { isModelPromptDue } from '../startup-prompts.helpers';

// Plumed fork (written with Claude Code): asks once whether to share anonymous
// usage statistics. Waits for a launch without the model download or remote
// microphone question so the dialogs never stack.
export const TelemetryPrompt = () => {
    const { t } = useTranslation();
    const available = useTelemetryAvailable();
    const [open, setOpen] = useState(false);

    useEffect(() => {
        if (!available) {
            return;
        }
        Promise.all([
            invoke<AppSettings>('get_all_settings'),
            invoke<boolean>('remote_mic_prompt_due'),
            isModelPromptDue(),
        ])
            .then(([settings, remoteMicDue, modelDue]) => {
                setOpen(!settings.telemetry_asked && !remoteMicDue && !modelDue);
            })
            .catch((e) => console.error('Failed to check the usage statistics prompt:', e));
    }, [available]);

    const answer = async (enabled: boolean) => {
        setOpen(false);
        try {
            await invoke('set_telemetry_enabled', { enabled });
        } catch (e) {
            toast.error(t('Failed to save the usage statistics setting'));
            console.error(e);
        }
    };

    // Closing the dialog counts as "no".
    const handleOpenChange = (isOpen: boolean) => {
        if (!isOpen) {
            void answer(false);
        }
    };

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogContent className="max-w-lg" overlayClassName="backdrop-blur-sm" data-testid="telemetry-prompt">
                <DialogHeader className="space-y-3">
                    <div className="flex items-center gap-3">
                        <div className="w-8 h-8 bg-sky-500/15 rounded-full flex items-center justify-center flex-shrink-0">
                            <ChartNoAxesColumn className="w-4 h-4 text-sky-400" />
                        </div>
                        <DialogTitle>{t('Help improve Plumed?')}</DialogTitle>
                    </div>
                    <DialogDescription>
                        {t(
                            'Plumed can send anonymous usage statistics to its maintainer, to learn how it is used and fix what goes wrong. Nothing is sent unless you agree.'
                        )}
                    </DialogDescription>
                </DialogHeader>

                <TelemetryDetails />

                <DialogFooter>
                    <Page.SecondaryButton
                        autoFocus
                        onClick={() => void answer(false)}
                        data-testid="telemetry-prompt-no"
                    >
                        {t('No thanks')}
                    </Page.SecondaryButton>
                    <Page.PrimaryButton onClick={() => void answer(true)} data-testid="telemetry-prompt-yes">
                        {t('Share anonymous statistics')}
                    </Page.PrimaryButton>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};
