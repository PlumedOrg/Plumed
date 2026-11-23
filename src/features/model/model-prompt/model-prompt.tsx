import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Download } from 'lucide-react';
import { toast } from 'react-toastify';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/dialog';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { useModelStatus } from '../hooks/use-model-status';
import { ModelChooser } from '../model-chooser/model-chooser';
import { isModelMissing } from '../model.helpers';

// Plumed fork (written with Claude Code): the speech model is not bundled.
// While none is downloaded (first launch, deleted or damaged model) this
// dialog offers the catalog, before any other startup question. Recording
// without a model reopens it.
export const ModelPrompt = () => {
    const { t } = useTranslation();
    const { status, setStatus } = useModelStatus();
    const [dismissed, setDismissed] = useState(false);
    const missing = status != null && isModelMissing(status);

    useEffect(() => {
        const unlisten = listen('model-missing', () => {
            toast.error(t('No model downloaded'));
            setDismissed(false);
        });
        return () => {
            void unlisten.then((stop) => stop());
        };
    }, [t]);

    // A missing model after a download or a check: show the dialog again.
    useEffect(() => {
        if (missing) {
            setDismissed(false);
        }
    }, [missing]);

    if (status == null) {
        return null;
    }

    return (
        <Dialog open={missing && !dismissed} onOpenChange={(isOpen) => setDismissed(!isOpen)}>
            <DialogContent className="max-w-lg" overlayClassName="backdrop-blur-sm" data-testid="model-prompt">
                <DialogHeader className="space-y-3">
                    <div className="flex items-center gap-3">
                        <div className="w-8 h-8 bg-sky-500/15 rounded-full flex items-center justify-center flex-shrink-0">
                            <Download className="w-4 h-4 text-sky-400" />
                        </div>
                        <DialogTitle>{t('Choose a speech model')}</DialogTitle>
                    </div>
                    <DialogDescription>
                        {t(
                            'Plumed needs a speech model to transcribe. It runs on this computer: your recordings never leave it.'
                        )}
                    </DialogDescription>
                </DialogHeader>

                <ModelChooser status={status} onStatus={setStatus} />

                <Typography.Paragraph className="text-xs text-muted-foreground">
                    {t(
                        'The download happens once and is kept across updates. You can change the model later in Settings > System > Model.'
                    )}
                </Typography.Paragraph>
            </DialogContent>
        </Dialog>
    );
};
