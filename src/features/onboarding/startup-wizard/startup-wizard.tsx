import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useNavigate } from '@tanstack/react-router';
import { ArrowLeft } from 'lucide-react';
import { toast } from 'react-toastify';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/dialog';
import { Page } from '@/components/page';
import { useTranslation } from '@/i18n';
import { AppSettings } from '@/features/settings/settings.types';
import { getDueSteps, StartupStep } from '../startup-prompts.helpers';
import { MedicalDataStep } from './steps/medical-data-step';
import { ModelStep } from './steps/model-step';
import { RemoteMicStep } from './steps/remote-mic-step';
import { TelemetryStep } from './steps/telemetry-step';
import { UpdateChannelStep } from './steps/update-channel-step';

// Plumed fork (written with Claude Code): every first launch question in one
// dialog, one step per unanswered question, sliding sideways. A step shows only
// while its question is unanswered, so existing users are not asked again.
// The dialog cannot be dismissed: each step has its own way to answer or
// postpone. Once done, a user who never opened the tutorial lands on it.
export const StartupWizard = () => {
    const { t } = useTranslation();
    const navigate = useNavigate();
    const [steps, setSteps] = useState<StartupStep[]>([]);
    const [offerTelemetryNeverAgain, setOfferTelemetryNeverAgain] = useState(false);
    const [index, setIndex] = useState(0);
    const [open, setOpen] = useState(false);

    const openTutorialFirstTime = useCallback(() => {
        invoke<AppSettings>('get_all_settings')
            .then((settings) => {
                if (!settings.tutorial_opened) {
                    void navigate({ to: '/tutorial' });
                }
            })
            .catch((e) => console.error('Failed to check the tutorial:', e));
    }, [navigate]);

    useEffect(() => {
        getDueSteps()
            .then((due) => {
                setSteps(due.steps);
                setOfferTelemetryNeverAgain(due.offerTelemetryNeverAgain);
                if (due.steps.length > 0) {
                    setOpen(true);
                } else {
                    openTutorialFirstTime();
                }
            })
            .catch((e) => console.error('Failed to check the startup questions:', e));
    }, [openTutorialFirstTime]);

    // Recording without a model reopens the wizard on the model step.
    useEffect(() => {
        const unlisten = listen('model-missing', () => {
            toast.error(t('No model downloaded'));
            const modelIndex = steps.indexOf('model');
            if (open && modelIndex >= 0) {
                setIndex(modelIndex);
                return;
            }
            // Closed (or no model step in this run): only the model question.
            setSteps(['model']);
            setIndex(0);
            setOpen(true);
        });
        return () => {
            void unlisten.then((stop) => stop());
        };
    }, [t, steps, open]);

    const next = () => {
        if (index + 1 < steps.length) {
            setIndex(index + 1);
            return;
        }
        setOpen(false);
        openTutorialFirstTime();
    };

    const renderStep = (step: StartupStep) => {
        switch (step) {
            case 'model':
                return <ModelStep onNext={next} />;
            case 'remote-mic':
                return <RemoteMicStep onNext={next} />;
            case 'telemetry':
                return <TelemetryStep onNext={next} offerNeverAgain={offerTelemetryNeverAgain} />;
            case 'medical-data':
                return <MedicalDataStep onNext={next} />;
            case 'update-channel':
                return <UpdateChannelStep onNext={next} />;
        }
    };

    return (
        <Dialog open={open}>
            <DialogContent
                className="max-w-lg"
                overlayClassName="backdrop-blur-sm"
                hideCloseButton
                onEscapeKeyDown={(event) => event.preventDefault()}
                onInteractOutside={(event) => event.preventDefault()}
                data-testid="startup-wizard"
            >
                <DialogTitle className="sr-only">{t('Welcome to Plumed')}</DialogTitle>
                <DialogDescription className="sr-only">
                    {t('A few questions before you start. Each one can be changed later in the settings.')}
                </DialogDescription>
                <div className="overflow-hidden">
                    <div
                        className="flex items-start transition-transform duration-300 ease-in-out"
                        style={{ transform: `translateX(-${index * 100}%)` }}
                    >
                        {steps.map((step, position) => (
                            <div
                                key={step}
                                className="w-full shrink-0 px-1"
                                inert={position !== index}
                                aria-hidden={position !== index}
                            >
                                {renderStep(step)}
                            </div>
                        ))}
                    </div>
                </div>
                {steps.length > 1 && (
                    <div className="flex items-center justify-between text-xs text-muted-foreground">
                        <Page.SecondaryButton
                            onClick={() => setIndex(index - 1)}
                            disabled={index === 0}
                            className={index === 0 ? 'invisible' : undefined}
                            data-testid="wizard-back"
                        >
                            <ArrowLeft className="w-4 h-4" />
                            {t('Back')}
                        </Page.SecondaryButton>
                        <div className="flex items-center gap-2" data-testid="wizard-progress">
                            {steps.map((step, position) => (
                                <span
                                    key={step}
                                    className={`h-2 w-2 rounded-full ${position === index ? 'bg-sky-400' : 'bg-muted'}`}
                                />
                            ))}
                            <span>{`${index + 1} / ${steps.length}`}</span>
                        </div>
                    </div>
                )}
            </DialogContent>
        </Dialog>
    );
};
