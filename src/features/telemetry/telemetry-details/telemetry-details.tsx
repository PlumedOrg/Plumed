import { ReactNode, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Eye } from 'lucide-react';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { Button } from '@/components/button';

interface DetailsSectionProps {
    title: string;
    className: string;
    children: ReactNode;
}

const DetailsSection = ({ title, className, children }: DetailsSectionProps) => (
    <section className="space-y-1">
        <h4 className={`text-sm font-semibold ${className}`}>{title}</h4>
        <ul className="list-disc space-y-1 pl-5">{children}</ul>
    </section>
);

// Plumed fork (written with Claude Code): what the anonymous usage statistics
// contain, shared by the startup wizard and the settings toggle, with a
// button showing the exact JSON that would be sent now.
export const TelemetryDetails = () => {
    const { t } = useTranslation();
    const [preview, setPreview] = useState<string | null>(null);

    const togglePreview = async () => {
        if (preview !== null) {
            setPreview(null);
            return;
        }
        try {
            setPreview(await invoke<string>('get_telemetry_preview'));
        } catch (e) {
            toast.error(t('Failed to build the usage statistics preview'));
            console.error(e);
        }
    };

    return (
        <div className="space-y-3 text-sm text-muted-foreground">
            <DetailsSection title={t('Collected')} className="text-sky-400">
                <li>
                    {t(
                        'Aggregated counts, once a day on days you use Plumed: number of dictations, average length, errors, app version, operating system, which features are on.'
                    )}
                </li>
                <li>
                    {t(
                        'A random identifier, renewed every 13 months, tells days of the same installation apart.'
                    )}
                </li>
            </DetailsSection>
            <DetailsSection title={t('Never collected')} className="text-emerald-400">
                <li>{t('No audio and no transcription.')}</li>
                <li>
                    {t(
                        'Not your personal rules (formatting rules, personal dictionary, edited medical rules), only their approximate number.'
                    )}
                </li>
                <li>{t('It cannot be used to train a model or to build a dataset.')}</li>
                <li>{t('Shared with no other person or organization.')}</li>
                <li>{t('Part of no economic activity, in particular no advertising profiling.')}</li>
            </DetailsSection>
            <DetailsSection title={t('In all cases')} className="text-foreground">
                <li>{t('Optional, and you can change it at any time.')}</li>
                <li>
                    {t('Self-hosted by the maintainer in France with')}{' '}
                    <a
                        href="https://umami.is"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="underline hover:text-foreground"
                    >
                        Umami
                    </a>
                    .
                </li>
                <li>{t('Follows the CNIL guidance, purged after 25 months.')}</li>
                <li>
                    {t(
                        'Your IP address is never recorded in the statistics: it stays at most 24 hours in a technical log, then is erased.'
                    )}
                </li>
                <li>{t('The sending time is drawn at random.')}</li>
            </DetailsSection>
            <Button
                variant="subtle"
                size="sm"
                onClick={() => void togglePreview()}
                data-testid="telemetry-preview-button"
            >
                <Eye />
                {t('See what would be sent')}
            </Button>
            {preview !== null && (
                <pre
                    className="max-h-60 overflow-auto rounded-md border border-border bg-background p-2 font-mono text-xs text-foreground"
                    data-testid="telemetry-preview"
                >
                    {preview}
                </pre>
            )}
        </div>
    );
};
