import { useState } from 'react';
import { FileAudio, Mic, RadioTower, Square, Trash2 } from 'lucide-react';
import { Button } from '@/components/button';
import { Page } from '@/components/page';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { Meeting } from '../hooks/use-meeting';
import { formatDuration } from '../meeting.helpers';
import { MeetingSource } from '../meeting.types';

const SOURCES: { id: MeetingSource; icon: typeof Mic; label: string }[] = [
    { id: 'mic', icon: Mic, label: 'Microphone' },
    { id: 'phone', icon: RadioTower, label: 'PhoneMic' },
    { id: 'file', icon: FileAudio, label: 'Audio file' },
];

// Plumed fork (written with Claude Code): source, start / stop, progress and
// "new meeting" of the Réunion page. Starting again continues the meeting.
export const MeetingControls = ({ meeting }: { meeting: Meeting }) => {
    const { t } = useTranslation();
    const [source, setSource] = useState<MeetingSource>('mic');
    const [confirmingReset, setConfirmingReset] = useState(false);
    const { running, starting, stopping, file, processedSecs, lines } = meeting;
    const busy = running || starting;
    const hasMeeting = lines.length > 0 || processedSecs > 0;

    const fileShare =
        file?.duration != null && file.duration > 0 ? Math.min(1, (processedSecs - file.base) / file.duration) : null;

    const reset = () => {
        if (!confirmingReset) {
            setConfirmingReset(true);
            return;
        }
        setConfirmingReset(false);
        void meeting.reset();
    };

    return (
        <div className="rounded-md border border-border bg-background p-4 space-y-4">
            <div className="flex flex-wrap items-center gap-2">
                {SOURCES.map(({ id, icon: Icon, label }) => (
                    <Button
                        key={id}
                        variant={source === id ? 'default' : 'outline'}
                        size="sm"
                        disabled={busy}
                        onClick={() => setSource(id)}
                        data-testid={`meeting-source-${id}`}
                    >
                        <Icon />
                        {t(label)}
                    </Button>
                ))}
            </div>
            {source === 'phone' && !busy && (
                <Typography.Paragraph className="text-xs text-muted-foreground">
                    {t(
                        'Connect your phone on the PhoneMic page first. Each recording made on the phone is then added to the meeting.'
                    )}
                </Typography.Paragraph>
            )}
            <div className="flex flex-wrap items-center gap-3">
                {running && !stopping ? (
                    <Page.PrimaryButton onClick={() => void meeting.stop()} data-testid="meeting-stop">
                        <Square />
                        {t('Stop')}
                    </Page.PrimaryButton>
                ) : (
                    <Page.PrimaryButton
                        onClick={() => void meeting.start(source)}
                        disabled={busy}
                        data-testid="meeting-start"
                    >
                        {source === 'file' ? <FileAudio /> : <Mic />}
                        {hasMeeting ? t('Continue the meeting') : t('Start the meeting')}
                    </Page.PrimaryButton>
                )}
                {hasMeeting && !busy && (
                    <Page.SecondaryButton onClick={reset} data-testid="meeting-reset">
                        <Trash2 />
                        {confirmingReset ? t('Click again to erase the transcript') : t('New meeting')}
                    </Page.SecondaryButton>
                )}
            </div>
            {(running || hasMeeting) && (
                <div className="space-y-1" data-testid="meeting-progress">
                    {fileShare != null && running && (
                        <div className="h-2 w-full overflow-hidden rounded bg-surface-border">
                            <div
                                className="h-full bg-sky-400 transition-[width]"
                                style={{ width: `${fileShare * 100}%` }}
                            />
                        </div>
                    )}
                    <Typography.Paragraph className="text-xs text-muted-foreground">
                        {t('Transcribed: {{time}}', { time: formatDuration(processedSecs) })}
                        {stopping && ` · ${t('Finishing the transcription...')}`}
                        {running && !stopping && ` · ${t('Speakers appear about 30 s after they speak.')}`}
                    </Typography.Paragraph>
                </div>
            )}
        </div>
    );
};
