import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/tooltip';
import { Typography } from '@/components/typography';
import { Gauge, Hourglass } from 'lucide-react';
import { useGetStatistic } from './hooks/use-get-statistic';
import { computeTypingMultiplier, formatResetMonths, formatTimeSaved } from './statistics.helpers';
import { StatCard } from './stat-card/stat-card';
import { useTranslation } from '@/i18n';
import { LocalPill } from '../local-pill/local-pill';

export const Statistics = () => {
    const { wpm, words, timeSavedSeconds, nextResetTimestamp } = useGetStatistic();

    // Plumed: the local card always shows; the stats wait for a first dictation.
    const hasDictated = wpm > 0 || words > 0 || timeSavedSeconds > 0;

    return (
        <div className="flex gap-3">
            <LocalPill />
            {hasDictated && (
                <DictationStats wpm={wpm} timeSavedSeconds={timeSavedSeconds} nextResetTimestamp={nextResetTimestamp} />
            )}
        </div>
    );
};

interface DictationStatsProps {
    wpm: number;
    timeSavedSeconds: number;
    nextResetTimestamp: number | null;
}

const DictationStats = ({ wpm, timeSavedSeconds, nextResetTimestamp }: DictationStatsProps) => {
    const { t, i18n } = useTranslation();
    const multiplier = computeTypingMultiplier(wpm);

    return (
        <>
            <Tooltip>
                <TooltipTrigger asChild>
                    <StatCard
                        tabIndex={0}
                        icon={Hourglass}
                        value={formatTimeSaved(timeSavedSeconds)}
                        label={t('gained this month')}
                    />
                </TooltipTrigger>
                <TooltipContent>
                    <Typography.Paragraph className="text-white text-xs max-w-64">
                        {t('The time you saved this month by dictating instead of typing on a keyboard.')}
                        {nextResetTimestamp != null && (
                            <>
                                <br />
                                <br />
                                {t(
                                    'Since {{month}} 1. Resets on {{nextMonth}} 1.',
                                    formatResetMonths(nextResetTimestamp, i18n.language)
                                )}
                            </>
                        )}
                    </Typography.Paragraph>
                </TooltipContent>
            </Tooltip>
            <Tooltip>
                <TooltipTrigger asChild>
                    <StatCard tabIndex={0} icon={Gauge} value={wpm > 0 ? wpm.toFixed(0) : '-'} label={t('wpm')} />
                </TooltipTrigger>
                <TooltipContent>
                    <Typography.Paragraph className="text-white text-xs max-w-64">
                        {t('Your average words per minute with Murmure since you started.')}
                        {multiplier != null && <> {t('That is {{multiplier}} faster than typing.', { multiplier })}</>}
                        <br />
                        <br />
                        {t('A fast keyboard user usually types around 80 words per minute. You can speak much faster.')}
                    </Typography.Paragraph>
                </TooltipContent>
            </Tooltip>
        </>
    );
};
