// Plumed fork (written with Claude Code): what the model screens show.
import { formatData } from '@/features/home/statistics/statistics.helpers';
import { ModelDownloadError, ModelDownloadProgress, ModelSource, ModelStatus } from './types';

type Translate = (key: string, options?: Record<string, string>) => string;

/** No model to transcribe with: the download dialog must open. */
export const isModelMissing = (status: ModelStatus): boolean => status.active == null && !status.dev;

/** A size in bytes, for example "668 MB". */
export const formatBytes = (bytes: number, t: Translate, language: string): string =>
    formatData(bytes / 1_000_000, language, { MB: t('MB'), GB: t('GB'), TB: t('TB') });

/** Seconds left at the current speed, null while it is unknown. */
export const secondsLeft = (progress: ModelDownloadProgress): number | null => {
    if (progress.speed <= 0) {
        return null;
    }
    return Math.max(0, (progress.total - progress.downloaded) / progress.speed);
};

/** "12 s left", "about 4 min left". */
export const formatEta = (seconds: number | null, t: Translate): string => {
    if (seconds == null) {
        return t('estimating the time left');
    }
    if (seconds < 60) {
        return t('{{seconds}} s left', { seconds: String(Math.ceil(seconds)) });
    }
    return t('about {{minutes}} min left', { minutes: String(Math.ceil(seconds / 60)) });
};

/** "45 % · 12.3 MB/s · about 4 min left". */
export const progressLabel = (progress: ModelDownloadProgress, t: Translate, language: string): string => {
    const percent = progress.total > 0 ? Math.floor((progress.downloaded * 100) / progress.total) : 0;
    const speed = `${formatBytes(progress.speed, t, language)}/s`;
    return `${percent} % · ${speed} · ${formatEta(secondsLeft(progress), t)}`;
};

export const sourceLabel = (source: ModelSource, t: Translate): string => {
    switch (source) {
        case 'hugging_face':
            return t('Hugging Face');
        case 'relay':
            return t('the Plumed relay (models.plumed.fr)');
    }
};

const isDownloadError = (error: unknown): error is ModelDownloadError =>
    typeof error === 'object' && error != null && 'kind' in error;

/** Why the download stopped, in the user's language. */
export const downloadErrorMessage = (error: unknown, t: Translate, language: string): string => {
    if (!isDownloadError(error)) {
        return String(error);
    }
    switch (error.kind) {
        case 'network':
            return t('Neither Hugging Face nor the Plumed relay could be reached ({{detail}}).', {
                detail: error.detail,
            });
        case 'disk_space':
            return t('Not enough disk space: {{needed}} needed, {{available}} free.', {
                needed: formatBytes(error.needed, t, language),
                available: formatBytes(error.available, t, language),
            });
        case 'checksum':
            return t('The file {{file}} kept arriving damaged.', { file: error.file });
        case 'cancelled':
            return t('Download cancelled.');
        case 'busy':
            return t('Another model is being downloaded.');
        case 'unavailable':
            return t('This model is not available yet.');
        case 'io':
            return t('The model could not be saved ({{detail}}).', { detail: error.detail });
    }
};
