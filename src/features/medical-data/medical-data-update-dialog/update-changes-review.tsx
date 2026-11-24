import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ChevronDown, ChevronRight, Loader2 } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { Typography } from '@/components/typography';
import { MedicalDataFileDiff } from '../types';

interface UpdateChangesReviewProps {
    version: string;
    /** True while the files are being downloaded for the review. */
    onBusyChange: (busy: boolean) => void;
}

// Plumed fork (written with Claude Code): optional, collapsed review of what a
// medical data update changes. Opening it downloads the files (kept for the
// install) and lists the lines added and removed per file.
export const UpdateChangesReview = ({ version, onBusyChange }: UpdateChangesReviewProps) => {
    const { t } = useTranslation();
    const [open, setOpen] = useState(false);
    const [diffs, setDiffs] = useState<MedicalDataFileDiff[] | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [loading, setLoading] = useState(false);

    const toggle = async () => {
        setOpen(!open);
        if (open || diffs != null || loading) {
            return;
        }
        setLoading(true);
        setError(null);
        onBusyChange(true);
        try {
            setDiffs(await invoke<MedicalDataFileDiff[]>('preview_medical_data_update', { version }));
        } catch (e) {
            setError(String(e));
        } finally {
            setLoading(false);
            onBusyChange(false);
        }
    };

    const Chevron = open ? ChevronDown : ChevronRight;
    return (
        <div className="space-y-2">
            <button
                type="button"
                className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
                onClick={() => void toggle()}
                data-testid="medical-data-update-review"
            >
                <Chevron className="h-4 w-4" />
                {t('Review the changes')}
            </button>
            {open && (
                <div className="max-h-64 overflow-y-auto rounded border border-border p-2 space-y-3 text-xs">
                    {loading && <Loader2 className="h-4 w-4 animate-spin" />}
                    {error != null && <Typography.Paragraph className="text-red-400">{error}</Typography.Paragraph>}
                    {diffs != null && diffs.length === 0 && (
                        <Typography.Paragraph>
                            {t('No line changes compared with the rules in use.')}
                        </Typography.Paragraph>
                    )}
                    {diffs?.map((diff) => (
                        <div key={diff.name}>
                            <div className="font-medium">
                                {diff.name} <span className="text-green-400">+{diff.added}</span>{' '}
                                <span className="text-red-400">-{diff.removed}</span>
                            </div>
                            <pre className="whitespace-pre-wrap break-all font-mono">
                                {diff.added_samples.map((line) => (
                                    <div key={`+${line}`} className="text-green-400">
                                        + {line}
                                    </div>
                                ))}
                                {diff.removed_samples.map((line) => (
                                    <div key={`-${line}`} className="text-red-400">
                                        - {line}
                                    </div>
                                ))}
                            </pre>
                            {diff.added + diff.removed > diff.added_samples.length + diff.removed_samples.length && (
                                <div className="text-muted-foreground">{t('Only the first lines are shown.')}</div>
                            )}
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
};
