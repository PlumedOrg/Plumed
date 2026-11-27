import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { progressLabel, sourceLabel } from '../model.helpers';
import { ModelDownloadProgress as Progress } from '../types';

interface ModelDownloadProgressProps {
    /** Null until the first progress event: still connecting. */
    progress: Progress | null;
}

// Plumed fork (written with Claude Code): the progress bar of a running model
// download, shared by the model chooser and the first launch download dialog.
export const ModelDownloadProgress = ({ progress }: ModelDownloadProgressProps) => {
    const { t, i18n } = useTranslation();
    if (progress == null) {
        return <Typography.Paragraph className="text-xs">{t('Connecting...')}</Typography.Paragraph>;
    }
    const percent = progress.total > 0 ? (progress.downloaded * 100) / progress.total : 0;
    return (
        <div className="space-y-1" data-testid="model-progress">
            <div className="h-2 w-full overflow-hidden rounded bg-surface-border">
                <div className="h-full bg-sky-400 transition-[width]" style={{ width: `${percent}%` }} />
            </div>
            <Typography.Paragraph className="text-xs">
                {progressLabel(progress, t, i18n.language)} ({t('from')} {sourceLabel(progress.source, t)})
            </Typography.Paragraph>
        </div>
    );
};
