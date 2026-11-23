import { ShieldCheck } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/tooltip';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { StatCard } from '../statistics/stat-card/stat-card';

// Plumed fork (written with Claude Code): the "100 % local" card, first of the
// home page statistics row and styled like them.
export const LocalPill = () => {
    const { t } = useTranslation();

    return (
        <Tooltip>
            <TooltipTrigger asChild>
                <StatCard tabIndex={0} data-testid="local-pill" icon={ShieldCheck} value="100 %" label={t('local')} />
            </TooltipTrigger>
            <TooltipContent>
                <Typography.Paragraph className="text-white text-xs max-w-64">
                    {t('Neither the sound nor the transcription leaves your computer')}
                </Typography.Paragraph>
            </TooltipContent>
        </Tooltip>
    );
};
