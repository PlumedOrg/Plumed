import { useState } from 'react';
import { Check, FlaskConical, Loader2, QrCode, ShieldCheck, Smartphone, X } from 'lucide-react';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { Page } from '@/components/page';
import { useTranslation } from '@/i18n';
import { RemoteMicSnapshot } from '../remote-mic.types';

interface RemoteMicSessionProps {
    snapshot: RemoteMicSnapshot;
    start: () => void;
    stop: () => void;
    confirm: (fingerprint: string, accepted: boolean) => void;
}

/** The click only binds the phone it was shown for: `fingerprint` identifies it. */
const Candidate = ({
    fingerprint,
    confirm,
}: {
    fingerprint: string;
    confirm: (fingerprint: string, accepted: boolean) => void;
}) => {
    const { t } = useTranslation();
    return (
        <div className="flex flex-col items-center gap-4">
            <Typography.Paragraph className="text-center">
                {t('A phone scanned the QR code. Connect it?')}
            </Typography.Paragraph>
            <div className="flex gap-3">
                <Page.PrimaryButton size="sm" onClick={() => confirm(fingerprint, true)}>
                    <Check className="w-4 h-4" />
                    {t('Connect')}
                </Page.PrimaryButton>
                <Page.SecondaryButton size="sm" onClick={() => confirm(fingerprint, false)}>
                    <X className="w-4 h-4" />
                    {t('Refuse')}
                </Page.SecondaryButton>
            </div>
        </div>
    );
};

const PairingQr = ({ qr, hint }: { qr: string | null; hint: string }) => (
    <>
        {qr != null && (
            <img src={qr} alt="QR code" className="w-[220px] h-[220px] rounded-lg border border-border bg-white" />
        )}
        <Typography.Paragraph className="text-center">{hint}</Typography.Paragraph>
    </>
);

/** Hidden behind a click: a QR left on screen while a phone is connected is
 * one photo away from replacing that phone (the desktop still asks to confirm). */
const SwitchPhone = ({ qr }: { qr: string | null }) => {
    const { t } = useTranslation();
    const [shown, setShown] = useState(false);
    if (!shown) {
        return (
            <Page.SecondaryButton size="sm" onClick={() => setShown(true)}>
                <QrCode className="w-4 h-4" />
                {t('Pair another phone')}
            </Page.SecondaryButton>
        );
    }
    return <PairingQr qr={qr} hint={t('To switch to another phone, scan this QR code with it.')} />;
};

const Body = ({ snapshot, start, stop, confirm }: RemoteMicSessionProps) => {
    const { t } = useTranslation();
    const stopButton = (
        <Page.SecondaryButton size="sm" onClick={stop}>
            {t('Disconnect')}
        </Page.SecondaryButton>
    );

    switch (snapshot.status) {
        case 'stopped':
            return (
                <>
                    {snapshot.error != null && (
                        <Typography.Paragraph className="text-center text-yellow-300/90">
                            {t(snapshot.error, snapshot.error_params ?? {})}
                        </Typography.Paragraph>
                    )}
                    <Page.PrimaryButton onClick={start}>
                        <Smartphone className="w-4 h-4" />
                        {t('Connect a phone')}
                    </Page.PrimaryButton>
                </>
            );
        case 'starting':
            return (
                <Typography.Paragraph className="flex items-center gap-2">
                    <Loader2 className="w-4 h-4 animate-spin" />
                    {t('Connecting to the relay')}
                </Typography.Paragraph>
            );
        case 'waiting':
            return (
                <>
                    <PairingQr qr={snapshot.qr} hint={t('Scan this QR code with the phone camera.')} />
                    {stopButton}
                </>
            );
        case 'verify':
            return (
                <>
                    {snapshot.fingerprint != null && (
                        <Candidate fingerprint={snapshot.fingerprint} confirm={confirm} />
                    )}
                    {stopButton}
                </>
            );
        case 'connected':
            return (
                <>
                    <Typography.Paragraph className="flex items-center gap-2 text-center">
                        <ShieldCheck className="w-5 h-5 text-green-400" />
                        {t('Phone connected. Dictate on the phone: the text is pasted here.')}
                    </Typography.Paragraph>
                    <SwitchPhone qr={snapshot.qr} />
                    {stopButton}
                </>
            );
    }
};

export const RemoteMicSession = (props: RemoteMicSessionProps) => {
    const { t } = useTranslation();
    return (
        <SettingsUI.Section title={t('Connect a phone')} icon={QrCode}>
            <div className="flex flex-col items-center gap-4 p-6">
                <Typography.Paragraph className="flex items-center gap-2 text-center text-yellow-300/90">
                    <FlaskConical className="w-4 h-4 shrink-0" />
                    {t('Experimental: this feature is a pre-release. Expect bugs, and update Plumed when asked.')}
                </Typography.Paragraph>
                <Body {...props} />
            </div>
        </SettingsUI.Section>
    );
};
