import { TriangleAlert } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/dialog';
import { Page } from '@/components/page';

interface ExperimentalModelDialogProps {
    /** The experimental variant the user clicked, null when closed. */
    name: string | null;
    onKeep: () => void;
    onChooseAnyway: () => void;
}

// Plumed fork (written with Claude Code): the warning shown when an
// experimental model (UltiMed v2) is picked in the model chooser.
export const ExperimentalModelDialog = ({ name, onKeep, onChooseAnyway }: ExperimentalModelDialogProps) => {
    const { t } = useTranslation();
    return (
        <Dialog
            open={name != null}
            onOpenChange={(isOpen) => {
                if (!isOpen) {
                    onKeep();
                }
            }}
        >
            <DialogContent className="max-w-lg" overlayClassName="backdrop-blur-sm" data-testid="experimental-model">
                <DialogHeader className="space-y-3">
                    <div className="flex items-center gap-3">
                        <div className="w-8 h-8 bg-amber-500/15 rounded-full flex items-center justify-center flex-shrink-0">
                            <TriangleAlert className="w-4 h-4 text-amber-400" />
                        </div>
                        <DialogTitle>{t('Experimental model')}</DialogTitle>
                    </div>
                    <DialogDescription>
                        {t(
                            '{{name}} is an experimental version: it often produces much worse results than UltiMed v1. Stick to UltiMed v1 until v2 is fixed or v3 is released.',
                            { name: name ?? '' }
                        )}
                    </DialogDescription>
                </DialogHeader>
                <DialogFooter>
                    <Page.SecondaryButton onClick={onChooseAnyway} data-testid="experimental-model-choose">
                        {t('Choose it anyway')}
                    </Page.SecondaryButton>
                    <Page.PrimaryButton autoFocus onClick={onKeep} data-testid="experimental-model-keep">
                        {t('Stick to UltiMed v1')}
                    </Page.PrimaryButton>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};
