import { Download } from 'lucide-react';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/dialog';
import { Page } from '@/components/page';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { ModelDownload } from '@/features/model/hooks/use-model-download';
import { ModelDownloadProgress } from '@/features/model/model-download-progress/model-download-progress';
import { downloadErrorMessage } from '@/features/model/model.helpers';

interface DownloadDialogProps {
    open: boolean;
    download: ModelDownload;
    onRetry: () => void;
    /** Back to the model step of the wizard (after cancelling, or on error). */
    onChooseAnother: () => void;
}

// Plumed fork (written with Claude Code): once the first launch questions are
// answered, the model download they started blocks the app until it is done,
// so nobody tries to dictate without a model. It cannot be dismissed: it
// closes on success, and offers Retry or another model on failure.
export const DownloadDialog = ({ open, download, onRetry, onChooseAnother }: DownloadDialogProps) => {
    const { t, i18n } = useTranslation();
    const failed = !download.running && download.error != null;

    return (
        <Dialog open={open}>
            <DialogContent
                className="max-w-lg"
                overlayClassName="backdrop-blur-sm"
                hideCloseButton
                onEscapeKeyDown={(event) => event.preventDefault()}
                onInteractOutside={(event) => event.preventDefault()}
                data-testid="model-download-dialog"
            >
                <DialogTitle className="flex items-center gap-2">
                    <Download className="w-5 h-5 text-sky-400" />
                    {t('Downloading the speech model')}
                </DialogTitle>
                <DialogDescription>
                    {t('Plumed can transcribe once the model is downloaded. This happens only once.')}
                </DialogDescription>
                {download.running && <ModelDownloadProgress progress={download.progress} />}
                {failed && (
                    <Typography.Paragraph className="text-sm text-red-400" data-testid="model-download-error">
                        {downloadErrorMessage(download.error, t, i18n.language)}
                    </Typography.Paragraph>
                )}
                <div className="flex justify-end gap-2">
                    {download.running && (
                        <Page.SecondaryButton onClick={download.cancel} data-testid="model-download-cancel">
                            {t('Cancel')}
                        </Page.SecondaryButton>
                    )}
                    {failed && (
                        <>
                            <Page.SecondaryButton onClick={onChooseAnother} data-testid="model-download-choose">
                                {t('Choose another model')}
                            </Page.SecondaryButton>
                            <Page.PrimaryButton onClick={onRetry} data-testid="model-download-retry">
                                {t('Retry')}
                            </Page.PrimaryButton>
                        </>
                    )}
                </div>
            </DialogContent>
        </Dialog>
    );
};
