import { useState } from 'react';
import { Check, Copy } from 'lucide-react';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { Switch } from '@/components/switch';
import { Typography } from '@/components/typography';
import { useCopyFeedback } from '@/components/hooks/use-copy-feedback';
import { useTranslation } from '@/i18n';
import { formatTranscript, lineTimestamp, speakerLetter, speakerName } from '../meeting.helpers';
import { MeetingLine, TimestampOptions } from '../meeting.types';

const SPEAKER_COLORS = ['text-sky-400', 'text-amber-400', 'text-emerald-400', 'text-fuchsia-400'];

// Plumed fork (written with Claude Code): the transcript, "- Name: text" per
// speaker turn, with renamable speakers and optional timestamps.
export const MeetingTranscript = ({ lines }: { lines: MeetingLine[] }) => {
    const { t } = useTranslation();
    const [names, setNames] = useState<string[]>([]);
    const [timestamps, setTimestamps] = useState<TimestampOptions>({ enabled: false, startClock: '', offsetSecs: 0 });
    const text = formatTranscript(lines, names, timestamps);
    const { copied, handleCopy } = useCopyFeedback(text);
    const speakers = [...new Set(lines.map((line) => line.speaker))].sort((a, b) => a - b);

    if (lines.length === 0) {
        return null;
    }
    return (
        <div className="rounded-md border border-border bg-background p-4 space-y-4" data-testid="meeting-transcript">
            <div className="flex flex-wrap items-center gap-3">
                {speakers.map((speaker) => (
                    <label key={speaker} className="flex items-center gap-2 text-sm">
                        <span className={SPEAKER_COLORS[speaker % SPEAKER_COLORS.length]}>
                            {speakerLetter(speaker)}
                        </span>
                        <Input
                            className="h-8 w-36"
                            value={names[speaker] ?? speakerLetter(speaker)}
                            aria-label={t('Name of speaker {{speaker}}', { speaker: speakerLetter(speaker) })}
                            onChange={(e) =>
                                setNames((previous) => {
                                    const next = [...previous];
                                    next[speaker] = e.target.value;
                                    return next;
                                })
                            }
                            data-testid={`meeting-speaker-${speaker}`}
                        />
                    </label>
                ))}
            </div>
            <div className="flex flex-wrap items-center gap-3 text-sm">
                <label className="flex items-center gap-2">
                    <Switch
                        checked={timestamps.enabled}
                        onCheckedChange={(enabled) => setTimestamps((previous) => ({ ...previous, enabled }))}
                        data-testid="meeting-timestamps"
                    />
                    {t('Timestamps')}
                </label>
                {timestamps.enabled && (
                    <>
                        <label className="flex items-center gap-2">
                            {t('Start time')}
                            <Input
                                className="h-8 w-24"
                                placeholder="14:00"
                                value={timestamps.startClock}
                                onChange={(e) =>
                                    setTimestamps((previous) => ({ ...previous, startClock: e.target.value }))
                                }
                            />
                        </label>
                        <label className="flex items-center gap-2">
                            {t('Offset (seconds)')}
                            <Input
                                className="h-8 w-24"
                                type="number"
                                value={timestamps.offsetSecs}
                                onChange={(e) =>
                                    setTimestamps((previous) => ({
                                        ...previous,
                                        offsetSecs: Number(e.target.value) || 0,
                                    }))
                                }
                            />
                        </label>
                    </>
                )}
                <Button variant="outline" size="sm" className="ml-auto" onClick={handleCopy} data-testid="meeting-copy">
                    {copied ? <Check className="text-emerald-400" /> : <Copy />}
                    {t('Copy')}
                </Button>
            </div>
            <div className="space-y-1 text-sm select-text">
                {lines.map((line, index) => (
                    <p key={index}>
                        {'- '}
                        {timestamps.enabled && (
                            <span className="text-muted-foreground">[{lineTimestamp(line.start, timestamps)}] </span>
                        )}
                        <span className={SPEAKER_COLORS[line.speaker % SPEAKER_COLORS.length]}>
                            {speakerName(names, line.speaker)}
                        </span>
                        {`: ${line.text}`}
                    </p>
                ))}
            </div>
            {text.length > 0 && (
                <Typography.Paragraph className="text-xs text-muted-foreground">
                    {t('{{count}} lines', { count: lines.length })}
                </Typography.Paragraph>
            )}
        </div>
    );
};
