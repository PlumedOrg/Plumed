import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Lock, Server, TriangleAlert } from 'lucide-react';
import { toast } from 'react-toastify';
import clsx from 'clsx';
import { useTranslation } from '@/i18n';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/dialog';
import { ExternalLink } from '@/components/external-link';
import { Input } from '@/components/input';
import { Page } from '@/components/page';
import { HDS_INFO_URL, PLUMED_RELAY_URL } from '@/utils/plumed-links';

interface RemoteMicRelayDialogProps {
    open: boolean;
    onCancel: () => void;
    /** Called once the chosen relay is saved, with its normalized origin. */
    onConfirmed: (origin: string) => void;
}

interface RelayOptionProps {
    selected: boolean;
    onSelect: () => void;
    title: string;
    children?: React.ReactNode;
    testId: string;
}

const RelayOption = ({ selected, onSelect, title, children, testId }: RelayOptionProps) => (
    <div
        role="radio"
        aria-checked={selected}
        tabIndex={0}
        onClick={onSelect}
        onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') onSelect();
        }}
        data-testid={testId}
        className={clsx(
            'rounded-md border px-3 py-2 cursor-pointer space-y-2',
            selected ? 'border-sky-500 bg-sky-500/10' : 'border-border hover:border-sky-500/50'
        )}
    >
        <div className="text-sm font-medium text-foreground">{title}</div>
        {children}
    </div>
);

// Plumed fork: shown when the phone microphone is enabled. The user picks the
// relay (Plumed's, built in, or their own) after reading what it can see.
export const RemoteMicRelayDialog = ({ open, onCancel, onConfirmed }: RemoteMicRelayDialogProps) => {
    const { t } = useTranslation();
    const [custom, setCustom] = useState(false);
    const [customUrl, setCustomUrl] = useState('');

    const confirm = async () => {
        const url = custom ? customUrl.trim() : PLUMED_RELAY_URL;
        try {
            const origin = await invoke<string>('set_remote_mic_url', { url });
            onConfirmed(origin);
        } catch (e) {
            toast.error(t(String(e)));
        }
    };

    return (
        <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onCancel()}>
            <DialogContent
                className="max-w-lg"
                overlayClassName="backdrop-blur-sm"
                data-testid="remote-mic-relay-dialog"
            >
                <DialogHeader className="space-y-3">
                    <div className="flex items-center gap-3">
                        <div className="w-8 h-8 bg-sky-500/15 rounded-full flex items-center justify-center flex-shrink-0">
                            <Server className="w-4 h-4 text-sky-400" />
                        </div>
                        <DialogTitle>{t('Choose the relay server')}</DialogTitle>
                    </div>
                    <DialogDescription>
                        {t('The phone sends its audio to this computer through a relay server on the internet.')}
                    </DialogDescription>
                </DialogHeader>

                <div role="radiogroup" className="space-y-2">
                    <RelayOption
                        selected={!custom}
                        onSelect={() => setCustom(false)}
                        title={t('Plumed relay ({{host}})', { host: new URL(PLUMED_RELAY_URL).host })}
                        testId="remote-mic-relay-plumed"
                    />
                    <RelayOption
                        selected={custom}
                        onSelect={() => setCustom(true)}
                        title={t('Another relay, for example your institution’s own server')}
                        testId="remote-mic-relay-custom"
                    >
                        {custom && (
                            <Input
                                value={customUrl}
                                onChange={(e) => setCustomUrl(e.target.value)}
                                placeholder="https://relay.example.org"
                                autoFocus
                            />
                        )}
                    </RelayOption>
                </div>

                <div className="flex gap-2 rounded-md border border-border px-3 py-2 text-sm text-muted-foreground">
                    <Lock className="w-4 h-4 shrink-0 mt-0.5 text-sky-400" />
                    <span>
                        {t(
                            'The audio is encrypted on the phone before it leaves it. The relay only forwards data it cannot read and deletes it once delivered: only Plumed on this computer can decrypt it.'
                        )}
                    </span>
                </div>
                <div className="flex gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm text-amber-500">
                    <TriangleAlert className="w-4 h-4 shrink-0 mt-0.5" />
                    <span>
                        {t(
                            'The relay is not an HDS server (certified health data host). The phone microphone is an alpha feature: it may not be suitable for real patient data yet.'
                        )}{' '}
                        <ExternalLink href={HDS_INFO_URL}>{t('What is HDS?')}</ExternalLink>
                    </span>
                </div>

                <DialogFooter>
                    <Page.SecondaryButton onClick={onCancel} data-testid="remote-mic-relay-cancel">
                        {t('Cancel')}
                    </Page.SecondaryButton>
                    <Page.PrimaryButton
                        onClick={() => void confirm()}
                        disabled={custom && customUrl.trim() === ''}
                        data-testid="remote-mic-relay-confirm"
                    >
                        {t('Enable the remote microphone')}
                    </Page.PrimaryButton>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};
