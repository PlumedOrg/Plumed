import { useEffect, useRef, useState } from 'react';
import { Check, Copy } from 'lucide-react';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { Switch } from '@/components/switch';
import { Typography } from '@/components/typography';
import { useCopyFeedback } from '@/components/hooks/use-copy-feedback';
import { useTranslation } from '@/i18n';
import { formatTranscript, lineTimestamp, speakerLetter, speakerName } from '../meeting.helpers';
import { ClockAnchor, MeetingLine, TimestampOptions } from '../meeting.types';

const SPEAKER_COLORS = ['text-sky-400', 'text-amber-400', 'text-emerald-400', 'text-fuchsia-400'];

/** How close to the bottom (px) still counts as "reading the end": new lines then scroll into view. */
const FOLLOW_MARGIN = 40;

type TimestampSettings = Omit<TimestampOptions, 'anchors'>;

// Plumed fork (written with Claude Code): the transcript, "- Name: text" per
// speaker turn, with renamable speakers and optional timestamps. The lines
// scroll in their own box, which follows the new ones unless the user
// scrolled up to read.
export const MeetingTranscript = ({ lines, anchors }: { lines: MeetingLine[]; anchors: ClockAnchor[] }) => {
    const { t } = useTranslation();
    const [names, setNames] = useState<string[]>([]);
    const [settings, setTimestamps] = useState<TimestampSettings>({ enabled: false, startClock: '', offsetSecs: 0 });
    const timestamps: TimestampOptions = { ...settings, anchors };
    const text = formatTranscript(lines, names, timestamps);
    const scroller = useRef<HTMLDivElement>(null);
    const following = useRef(true);

    useEffect(() => {
        const box = scroller.current;
        if (box != null && following.current) box.scrollTop = box.scrollHeight;
    }, [lines]);
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
                                placeholder={t('auto')}
                                title={t(
                                    'Empty: the real time of each line for the microphone, the time since the start otherwise. Or the clock time of the start, like 14:00.'
                                )}
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
            <div
                ref={scroller}
                onScroll={(e) => {
                    const box = e.currentTarget;
                    following.current = box.scrollHeight - box.scrollTop - box.clientHeight < FOLLOW_MARGIN;
                }}
                className="max-h-[55vh] overflow-y-auto space-y-1 pr-2 text-sm select-text"
                data-testid="meeting-lines"
            >
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
