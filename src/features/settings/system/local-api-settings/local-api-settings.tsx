import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'react-toastify';
import { Copy, RefreshCw, Server } from 'lucide-react';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { Switch } from '@/components/switch';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { useTranslation } from '@/i18n';

interface LocalApiStatus {
    enabled: boolean;
    port: number;
    running_port: number | null;
    token: string;
}

/** Plumed fork: the OpenAI-compatible transcription API on 127.0.0.1 (off by default). */
export const LocalApiSettings = () => {
    const { t } = useTranslation();
    const [status, setStatus] = useState<LocalApiStatus | null>(null);
    const [port, setPort] = useState('');

    const apply = (next: LocalApiStatus) => {
        setStatus(next);
        setPort(String(next.port));
    };

    useEffect(() => {
        invoke<LocalApiStatus>('get_local_api_status')
            .then(apply)
            .catch(() => toast.error(t('Failed to load the local API settings')));
    }, [t]);

    // On failure (port taken), the backend turns the setting back off: reload it.
    const run = (command: string, args: Record<string, unknown> = {}) =>
        invoke<LocalApiStatus>(command, args)
            .then(apply)
            .catch((e) => {
                toast.error(String(e));
                invoke<LocalApiStatus>('get_local_api_status').then(apply);
            });

    const savePort = () => {
        const value = Number(port);
        if (!status || value === status.port) return;
        if (!Number.isInteger(value) || value < 1 || value > 65535) {
            toast.error(t('The port must be between 1 and 65535'));
            setPort(String(status.port));
            return;
        }
        run('set_local_api_port', { port: value });
    };

    const copyToken = async () => {
        if (!status) return;
        await navigator.clipboard.writeText(status.token);
        toast.info(t('API key copied'));
    };

    if (!status) return null;
    const curl = `curl http://127.0.0.1:${status.port}/v1/audio/transcriptions -H "Authorization: Bearer <KEY>" -F file=@dictee.wav`;

    return (
        <>
            <SettingsUI.Item>
                <SettingsUI.Description>
                    <Typography.Title className="flex items-center gap-2">
                        <Server className="w-4 h-4 text-muted-foreground" />
                        {t('Local transcription API')}
                    </Typography.Title>
                    <Typography.Paragraph>
                        {t(
                            'Serves an OpenAI-compatible transcription API on this computer only (127.0.0.1), protected by a key. It transcribes WAV files like a dictation, with your dictionary and rules, without adding them to the history. Leave it off unless a program of yours needs it. Also available without the window with "plumed serve".'
                        )}
                    </Typography.Paragraph>
                </SettingsUI.Description>
                <Switch
                    checked={status.enabled}
                    onCheckedChange={(enabled) => run('set_local_api_enabled', { enabled })}
                />
            </SettingsUI.Item>
            {status.enabled && (
                <div className="flex flex-col gap-2 px-4 pb-4 text-sm">
                    <div className="flex items-center gap-2">
                        <span className="w-24 text-muted-foreground">{t('Port')}</span>
                        <Input
                            className="w-28"
                            inputMode="numeric"
                            value={port}
                            onChange={(e) => setPort(e.target.value.replace(/\D/g, ''))}
                            onBlur={savePort}
                            onKeyDown={(e) => e.key === 'Enter' && savePort()}
                        />
                        <span className="text-muted-foreground">
                            {status.running_port === null
                                ? t('Stopped')
                                : t('Listening on http://127.0.0.1:{{port}}/v1', { port: status.running_port })}
                        </span>
                    </div>
                    <div className="flex items-center gap-2">
                        <span className="w-24 text-muted-foreground">{t('API key')}</span>
                        <code className="truncate rounded bg-muted px-2 py-1 text-xs">{status.token}</code>
                        <Button variant="outline" size="sm" onClick={copyToken} title={t('Copy')}>
                            <Copy className="w-4 h-4" />
                        </Button>
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={() => run('regenerate_local_api_token')}
                            title={t('Generate a new key (the old one stops working)')}
                        >
                            <RefreshCw className="w-4 h-4" />
                        </Button>
                    </div>
                    <Typography.Paragraph className="text-xs">
                        {t('Example:')} <code className="break-all">{curl}</code>
                    </Typography.Paragraph>
                </div>
            )}
        </>
    );
};
