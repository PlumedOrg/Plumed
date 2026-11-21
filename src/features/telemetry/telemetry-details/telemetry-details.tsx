import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Eye } from 'lucide-react';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { Button } from '@/components/button';

// Plumed fork (written with Claude Code): what the anonymous usage statistics
// contain, shared by the consent dialog and the settings toggle, with a
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
        <div className="space-y-2 text-sm text-muted-foreground">
            <p>
                {t(
                    'Collected: aggregated counts once a day on days you use Plumed (number of dictations, average length, errors, app version, operating system, which features are on). No text, no audio, no patient data.'
                )}
            </p>
            <p>
                {t(
                    'Never collected: what you dictate, your rules or dictionary words, file paths, computer or user names.'
                )}
            </p>
            <p>
                {t(
                    'Optional, off unless you say yes, and you can change it anytime in the settings. Hosted in the EU by the Plumed maintainer, kept at most 25 months. A random identifier, renewed every 13 months, tells days of the same installation apart.'
                )}
            </p>
            <Button
                variant="subtle"
                size="sm"
                onClick={() => void togglePreview()}
                data-testid="telemetry-preview-button"
            >
                <Eye />
                {t('See exactly what is sent')}
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
