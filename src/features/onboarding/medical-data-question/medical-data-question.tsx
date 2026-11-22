import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { DatabaseZap } from 'lucide-react';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/dialog';
import { Page } from '@/components/page';
import { isEarlierStartupPromptDue } from '../startup-prompts.helpers';

// Plumed fork (written with Claude Code): asks once whether Plumed may look
// for medical data updates at each start. No check, so no network call, is
// made before the answer. Waits for a launch without an earlier startup
// question so dialogs never stack; a "yes" checks from the next start on.
export const MedicalDataQuestion = () => {
    const { t } = useTranslation();
    const [open, setOpen] = useState(false);

    useEffect(() => {
        Promise.all([invoke<boolean>('medical_data_question_due'), isEarlierStartupPromptDue()])
            .then(([due, earlierDue]) => setOpen(due && !earlierDue))
            .catch((e) => console.error('Failed to check the medical data question:', e));
    }, []);

    const answer = async (enabled: boolean) => {
        setOpen(false);
        try {
            await invoke('set_medical_data_update_check', { enabled });
        } catch (e) {
            toast.error(t('Failed to save the medical data update setting'));
            console.error(e);
        }
    };

    // Closing the dialog counts as "no", like the usage statistics question.
    const handleOpenChange = (isOpen: boolean) => {
        if (!isOpen) {
            void answer(false);
        }
    };

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogContent className="max-w-lg" overlayClassName="backdrop-blur-sm" data-testid="medical-data-question">
                <DialogHeader className="space-y-3">
                    <div className="flex items-center gap-3">
                        <div className="w-8 h-8 bg-sky-500/15 rounded-full flex items-center justify-center flex-shrink-0">
                            <DatabaseZap className="w-4 h-4 text-sky-400" />
                        </div>
                        <DialogTitle>{t('Check for medical rules updates?')}</DialogTitle>
                    </div>
                    <DialogDescription>
                        {t(
                            'At each start, Plumed can check on GitHub whether newer medical correction lists are available: model correction rules, sound-alike word groups and medical vocabulary.'
                        )}
                    </DialogDescription>
                </DialogHeader>
                <ul className="list-disc pl-5 text-sm space-y-1">
                    <li>{t('Nothing is downloaded without asking you first.')}</li>
                    <li>{t('Nothing about you is sent: no identifier, no cookie, no usage data.')}</li>
                    <li>{t('You can change this choice in Settings > System > Online.')}</li>
                </ul>
                <DialogFooter>
                    <Page.SecondaryButton
                        autoFocus
                        onClick={() => void answer(false)}
                        data-testid="medical-data-question-no"
                    >
                        {t("Don't check")}
                    </Page.SecondaryButton>
                    <Page.PrimaryButton onClick={() => void answer(true)} data-testid="medical-data-question-yes">
                        {t('Check at each start')}
                    </Page.PrimaryButton>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};
