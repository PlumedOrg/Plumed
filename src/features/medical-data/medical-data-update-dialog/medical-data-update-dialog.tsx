import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { DatabaseZap, Loader2 } from 'lucide-react';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/dialog';
import { Page } from '@/components/page';
import { Typography } from '@/components/typography';
import { formatData } from '@/features/home/statistics/statistics.helpers';
import { useMedicalDataChangeToast } from '../hooks/use-medical-data-change-toast';
import { MedicalDataChange, MedicalDataOffer, MedicalDataProgress } from '../types';

interface MedicalDataUpdateDialogProps {
    /** The update to offer; the dialog is open while it is not null. */
    offer: MedicalDataOffer | null;
    /** Any answer: "Not now", "Skip this version", a finished download, or closing the dialog. */
    onClose: () => void;
    onInstalled?: (change: MedicalDataChange) => void;
}

// Plumed fork (written with Claude Code): offers a newer signed medical rules
// set. Nothing is downloaded until the user clicks "Download".
export const MedicalDataUpdateDialog = ({ offer, onClose, onInstalled }: MedicalDataUpdateDialogProps) => {
    const { t, i18n } = useTranslation();
    const showChange = useMedicalDataChangeToast();
    const [progress, setProgress] = useState<MedicalDataProgress | null>(null);
    const [downloading, setDownloading] = useState(false);

    useEffect(() => {
        const unlisten = listen<MedicalDataProgress>('medical-data-download-progress', (event) =>
            setProgress(event.payload)
        );
        return () => {
            void unlisten.then((stop) => stop());
        };
    }, []);

    const size = (bytes: number) =>
        formatData(bytes / 1_000_000, i18n.language, { MB: t('MB'), GB: t('GB'), TB: t('TB') });

    const download = async () => {
        if (offer == null) {
            return;
        }
        setDownloading(true);
        setProgress(null);
        try {
            const change = await invoke<MedicalDataChange>('download_medical_data_update', {
                version: offer.version,
            });
            showChange(change);
            onInstalled?.(change);
            onClose();
        } catch (e) {
            toast.error(`${t('Failed to download the medical rules')}: ${String(e)}`);
        } finally {
            setDownloading(false);
        }
    };

    const skip = () => {
        onClose();
        invoke('skip_medical_data_version', { version: offer?.version ?? '' }).catch((e) => {
            toast.error(t('Failed to save the medical rules update setting'));
            console.error(e);
        });
    };

    const percent =
        progress != null && progress.total > 0 ? Math.floor((progress.downloaded * 100) / progress.total) : 0;

    return (
        <Dialog
            open={offer != null}
            onOpenChange={(isOpen) => {
                if (!isOpen && !downloading) {
                    onClose();
                }
            }}
        >
            <DialogContent className="max-w-lg" overlayClassName="backdrop-blur-sm" data-testid="medical-data-update">
                <DialogHeader className="space-y-3">
                    <div className="flex items-center gap-3">
                        <div className="w-8 h-8 bg-sky-500/15 rounded-full flex items-center justify-center flex-shrink-0">
                            <DatabaseZap className="w-4 h-4 text-sky-400" />
                        </div>
                        <DialogTitle>{t('New medical rules available')}</DialogTitle>
                    </div>
                    <DialogDescription>
                        {t('Version {{version}} of the drug and medical term rules is available.', {
                            version: offer?.version ?? '',
                        })}
                    </DialogDescription>
                </DialogHeader>

                {offer != null && offer.notes.length > 0 && (
                    <Typography.Paragraph className="whitespace-pre-line">{offer.notes}</Typography.Paragraph>
                )}
                <Typography.Paragraph>
                    {t('{{count}} files, {{size}} to download.', {
                        count: offer?.file_count ?? 0,
                        size: size(offer?.total_size ?? 0),
                    })}
                </Typography.Paragraph>
                <Typography.Paragraph className="text-xs text-muted-foreground">
                    {t(
                        'Your own edits of the rules are kept. The files are signed by the Plumed maintainer and checked before use.'
                    )}
                </Typography.Paragraph>

                <DialogFooter>
                    <Page.SecondaryButton onClick={skip} disabled={downloading} data-testid="medical-data-update-skip">
                        {t('Skip this version')}
                    </Page.SecondaryButton>
                    <Page.SecondaryButton
                        autoFocus
                        onClick={onClose}
                        disabled={downloading}
                        data-testid="medical-data-update-later"
                    >
                        {t('Not now')}
                    </Page.SecondaryButton>
                    <Page.PrimaryButton
                        onClick={() => void download()}
                        disabled={downloading}
                        data-testid="medical-data-update-download"
                    >
                        {downloading && <Loader2 className="h-4 w-4 animate-spin mr-2" />}
                        {downloading ? `${percent} %` : t('Download')}
                    </Page.PrimaryButton>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};
