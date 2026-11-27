import { ReactNode, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Download } from 'lucide-react';
import { Page } from '@/components/page';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { useModelDownload } from '@/features/model/hooks/use-model-download';
import { ModelDownloadProgress } from '@/features/model/model-download-progress/model-download-progress';
import { downloadErrorMessage, formatBytes } from '@/features/model/model.helpers';
import { ModelVariantStatus } from '@/features/model/types';

const downloadDiarization = () => invoke<ModelVariantStatus>('download_diarization_model');

// Plumed fork (written with Claude Code): the meeting needs the speaker
// diarization model, downloaded on demand the first time.
export const MeetingModelGate = ({ children }: { children: ReactNode }) => {
    const { t, i18n } = useTranslation();
    const [status, setStatus] = useState<ModelVariantStatus | null>(null);
    const download = useModelDownload<ModelVariantStatus>(setStatus, downloadDiarization);

    useEffect(() => {
        invoke<ModelVariantStatus>('get_diarization_model_status').then(setStatus);
    }, []);

    if (status == null) {
        return null;
    }
    if (status.installed != null) {
        return <>{children}</>;
    }
    return (
        <div className="rounded-md border border-border bg-background p-4 space-y-3" data-testid="meeting-model-gate">
            <Typography.Paragraph>
                {t('To tell the speakers apart, the meeting mode needs a speaker recognition model ({{size}}). It runs on this computer.', {
                    size: formatBytes(status.size, t, i18n.language),
                })}
            </Typography.Paragraph>
            {download.running ? (
                <>
                    <ModelDownloadProgress progress={download.progress} />
                    <Page.SecondaryButton onClick={download.cancel}>{t('Cancel')}</Page.SecondaryButton>
                </>
            ) : (
                <Page.PrimaryButton onClick={() => void download.start(status.id)} data-testid="meeting-model-download">
                    <Download />
                    {t('Download')}
                </Page.PrimaryButton>
            )}
            {!download.running && download.error != null && (
                <Typography.Paragraph className="text-sm text-red-400">
                    {downloadErrorMessage(download.error, t, i18n.language)}
                </Typography.Paragraph>
            )}
        </div>
    );
};
