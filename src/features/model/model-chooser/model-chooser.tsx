import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import clsx from 'clsx';
import { Loader2 } from 'lucide-react';
import { toast } from 'react-toastify';
import { Button } from '@/components/button';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { ExperimentalModelDialog } from '../experimental-model-dialog/experimental-model-dialog';
import { useModelDownload } from '../hooks/use-model-download';
import { downloadErrorMessage, formatBytes, progressLabel, sourceLabel } from '../model.helpers';
import { ModelStatus, ModelVariantStatus } from '../types';

interface ModelChooserProps {
    status: ModelStatus;
    onStatus: (status: ModelStatus) => void;
}

const initialChoice = (status: ModelStatus): string =>
    status.active ?? status.downloading ?? status.variants.find((v) => v.installed != null)?.id ?? status.recommended;

// Plumed fork (written with Claude Code): the model catalog with the actions
// on the chosen variant (download, resume, cancel, use, check, delete). Shared
// by the first launch dialog and the settings.
export const ModelChooser = ({ status, onStatus }: ModelChooserProps) => {
    const { t, i18n } = useTranslation();
    const download = useModelDownload(onStatus);
    const [chosen, setChosen] = useState(() => initialChoice(status));
    const [busy, setBusy] = useState(false);
    const [confirmDelete, setConfirmDelete] = useState(false);
    const [warnFor, setWarnFor] = useState<ModelVariantStatus | null>(null);
    const downloading = download.running || status.downloading != null;
    const variant = status.variants.find((v) => v.id === chosen);

    const select = (id: string) => {
        setChosen(id);
        setConfirmDelete(false);
    };

    // Picking an experimental model asks first.
    const choose = (v: ModelVariantStatus) => {
        if (v.experimental && v.id !== chosen) {
            setWarnFor(v);
        } else {
            select(v.id);
        }
    };

    const whileBusy = async (job: () => Promise<void>) => {
        setBusy(true);
        try {
            await job();
        } finally {
            setBusy(false);
        }
    };

    const use = (id: string) =>
        whileBusy(async () => {
            try {
                onStatus(await invoke<ModelStatus>('set_active_model', { variant: id }));
            } catch (e) {
                toast.error(`${t('Failed to switch the model')}: ${String(e)}`);
            }
        });

    const verify = (id: string) =>
        whileBusy(async () => {
            try {
                const intact = await invoke<boolean>('verify_model', { variant: id });
                if (intact) {
                    toast.success(t('The model is intact.'));
                } else {
                    toast.error(t('The model was damaged: download it again (only the damaged files are fetched).'));
                }
            } catch (e) {
                toast.error(`${t('Failed to check the model')}: ${String(e)}`);
            }
        });

    const remove = (id: string) =>
        whileBusy(async () => {
            setConfirmDelete(false);
            try {
                onStatus(await invoke<ModelStatus>('delete_model', { variant: id }));
            } catch (e) {
                toast.error(`${t('Failed to delete the model')}: ${String(e)}`);
            }
        });

    const stateLabel = (v: ModelVariantStatus): string => {
        if (!v.available) {
            return t('coming soon');
        }
        if (v.id === status.active) {
            return t('in use');
        }
        if (v.installed != null) {
            return t('downloaded');
        }
        if (v.downloaded > 0) {
            return t('{{size}} already downloaded', { size: formatBytes(v.downloaded, t, i18n.language) });
        }
        return '';
    };

    const downloadLabel = (v: ModelVariantStatus): string => {
        if (download.error != null) {
            return t('Retry');
        }
        if (v.downloaded > 0) {
            return t('Resume the download');
        }
        return t('Download');
    };

    const actions = (v: ModelVariantStatus) => {
        if (downloading) {
            return (
                <Button variant="secondary" size="sm" onClick={download.cancel} data-testid="model-cancel">
                    {t('Cancel')}
                </Button>
            );
        }
        if (v.installed == null) {
            return (
                <Button
                    size="sm"
                    disabled={busy}
                    onClick={() => void download.start(v.id)}
                    data-testid="model-download"
                >
                    {downloadLabel(v)}
                </Button>
            );
        }
        return (
            <>
                {v.id !== status.active && (
                    <Button size="sm" disabled={busy} onClick={() => void use(v.id)} data-testid="model-use">
                        {t('Use this model')}
                    </Button>
                )}
                <Button
                    variant="secondary"
                    size="sm"
                    disabled={busy}
                    onClick={() => void verify(v.id)}
                    data-testid="model-verify"
                >
                    {busy && <Loader2 className="h-4 w-4 animate-spin" />}
                    {t('Check')}
                </Button>
                <Button
                    variant="secondary"
                    size="sm"
                    disabled={busy}
                    onClick={() => (confirmDelete ? void remove(v.id) : setConfirmDelete(true))}
                    data-testid="model-delete"
                >
                    {confirmDelete ? t('Confirm the deletion') : t('Delete')}
                </Button>
            </>
        );
    };

    return (
        <div className="space-y-3" data-testid="model-chooser">
            <ExperimentalModelDialog
                name={warnFor?.name ?? null}
                onKeep={() => setWarnFor(null)}
                onChooseAnyway={() => {
                    if (warnFor != null) {
                        select(warnFor.id);
                    }
                    setWarnFor(null);
                }}
            />
            <div className="space-y-2" role="radiogroup">
                {status.variants.map((v) => (
                    <button
                        key={v.id}
                        type="button"
                        role="radio"
                        aria-checked={v.id === chosen}
                        disabled={!v.available || downloading}
                        onClick={() => choose(v)}
                        className={clsx(
                            'w-full rounded-md border px-3 py-2 text-left text-sm transition-colors',
                            v.id === chosen ? 'border-sky-400 bg-sky-500/10' : 'border-surface-border',
                            !v.available && 'opacity-50 cursor-not-allowed'
                        )}
                        data-testid={`model-variant-${v.id}`}
                    >
                        <div className="flex items-center justify-between gap-2">
                            <span className="font-medium" title={v.family}>
                                {v.name}
                                {v.experimental && ` ${t('(EXPERIMENTAL)')}`}
                                {v.id === status.recommended && (
                                    <span className="ml-2 text-xs font-normal text-sky-400">{t('Recommended')}</span>
                                )}
                            </span>
                            <span className="text-xs text-muted-foreground">
                                {formatBytes(v.size, t, i18n.language)}
                            </span>
                        </div>
                        <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
                            <span>{t(v.summary)}</span>
                            <span>{stateLabel(v)}</span>
                        </div>
                    </button>
                ))}
            </div>

            {downloading && download.progress != null && (
                <div className="space-y-1" data-testid="model-progress">
                    <div className="h-2 w-full overflow-hidden rounded bg-surface-border">
                        <div
                            className="h-full bg-sky-400 transition-[width]"
                            style={{
                                width: `${download.progress.total > 0 ? (download.progress.downloaded * 100) / download.progress.total : 0}%`,
                            }}
                        />
                    </div>
                    <Typography.Paragraph className="text-xs">
                        {progressLabel(download.progress, t, i18n.language)} ({t('from')}{' '}
                        {sourceLabel(download.progress.source, t)})
                    </Typography.Paragraph>
                </div>
            )}
            {downloading && download.progress == null && (
                <Typography.Paragraph className="text-xs">{t('Connecting...')}</Typography.Paragraph>
            )}

            {!downloading && download.error != null && (
                <Typography.Paragraph className="text-sm text-red-400" data-testid="model-error">
                    {downloadErrorMessage(download.error, t, i18n.language)}
                </Typography.Paragraph>
            )}

            {variant?.installed != null && (
                <Typography.Paragraph className="text-xs">
                    {t('Downloaded from {{source}}.', { source: sourceLabel(variant.installed, t) })}
                </Typography.Paragraph>
            )}

            {variant != null && variant.available && <div className="flex flex-wrap gap-2">{actions(variant)}</div>}
        </div>
    );
};
