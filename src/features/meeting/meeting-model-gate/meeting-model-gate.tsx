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

const downloadMeetingModel = (variant: string) => invoke<ModelVariantStatus[]>('download_meeting_model', { variant });

// Plumed fork (written with Claude Code): the meeting needs the speaker
// diarization and voice models, downloaded on demand the first time.
export const MeetingModelGate = ({ children }: { children: ReactNode }) => {
    const { t, i18n } = useTranslation();
    const [models, setModels] = useState<ModelVariantStatus[] | null>(null);
    const download = useModelDownload<ModelVariantStatus[]>(setModels, downloadMeetingModel);

    useEffect(() => {
        invoke<ModelVariantStatus[]>('get_meeting_models_status').then(setModels);
    }, []);

    if (models == null) {
        return null;
    }
    const missing = models.filter((model) => model.installed == null);
    if (missing.length === 0) {
        return <>{children}</>;
    }
    // one after the other: the backend downloads a single model at a time
    const downloadMissing = async () => {
        for (const model of missing) {
            if (!(await download.start(model.id))) {
                return;
            }
        }
    };
    return (
        <div className="rounded-md border border-border bg-background p-4 space-y-3" data-testid="meeting-model-gate">
            <Typography.Paragraph>
                {t(
                    'To tell the speakers apart, the meeting mode needs speaker recognition models ({{size}}). They run on this computer.',
                    {
                        size: formatBytes(
                            missing.reduce((total, model) => total + model.size, 0),
                            t,
                            i18n.language
                        ),
                    }
                )}
            </Typography.Paragraph>
            {download.running ? (
                <>
                    <ModelDownloadProgress progress={download.progress} />
                    <Page.SecondaryButton onClick={download.cancel}>{t('Cancel')}</Page.SecondaryButton>
                </>
            ) : (
                <Page.PrimaryButton onClick={() => void downloadMissing()} data-testid="meeting-model-download">
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
