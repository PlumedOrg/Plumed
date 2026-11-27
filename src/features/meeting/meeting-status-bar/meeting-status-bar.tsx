import { Square } from 'lucide-react';
import { Button } from '@/components/button';
import { useTranslation } from '@/i18n';
import { Meeting } from '../hooks/use-meeting';
import { formatDuration } from '../meeting.helpers';

// Plumed fork (written with Claude Code): a bar pinned to the bottom of the
// Réunion page saying whether the meeting records, with its duration and a
// Stop button that stays reachable however long the transcript grows.
export const MeetingStatusBar = ({ meeting }: { meeting: Meeting }) => {
    const { t } = useTranslation();
    const { running, stopping, file, processedSecs, lines } = meeting;
    if (!running && lines.length === 0 && processedSecs === 0) return null;

    const [dot, label] = stopping
        ? ['bg-amber-400 animate-pulse', t('Finishing the transcription...')]
        : running
          ? file != null
              ? ['bg-sky-400 animate-pulse', t('Transcribing the file')]
              : ['bg-red-500 animate-pulse', t('Recording')]
          : ['bg-muted-foreground', t('Not recording')];

    return (
        <>
            {/* room under the transcript so the bar never hides its end */}
            <div aria-hidden="true" className="h-16" />
            <div className="pointer-events-none fixed inset-x-0 bottom-6 z-40 flex justify-center pl-[17.2rem] pr-8">
                <div
                    className="pointer-events-auto flex items-center gap-4 rounded-full border border-border bg-background/95 px-5 py-2 shadow-lg backdrop-blur"
                    data-testid="meeting-status-bar"
                >
                    <span className={`h-3 w-3 rounded-full ${dot}`} />
                    <span className="text-sm">{label}</span>
                    <span className="font-mono text-lg tabular-nums">{formatDuration(processedSecs)}</span>
                    {running && !stopping && (
                        <Button size="sm" variant="secondary" onClick={() => void meeting.stop()}>
                            <Square />
                            {t('Stop')}
                        </Button>
                    )}
                </div>
            </div>
        </>
    );
};
