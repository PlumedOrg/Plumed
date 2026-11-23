import { ShieldCheck } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/tooltip';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';

// Plumed fork (written with Claude Code): the "100 % local" pill of the home page.
export const LocalPill = () => {
    const { t } = useTranslation();

    return (
        <Tooltip>
            <TooltipTrigger asChild>
                <span
                    tabIndex={0}
                    data-testid="local-pill"
                    className="inline-flex w-fit cursor-default items-center gap-1.5 rounded-full border border-emerald-500/40 bg-emerald-500/10 px-3 py-1 text-xs font-semibold text-foreground outline-none focus-visible:border-emerald-400"
                >
                    <ShieldCheck className="h-4 w-4 text-emerald-400" aria-hidden="true" />
                    {t('100% local')}
                </span>
            </TooltipTrigger>
            <TooltipContent>
                <Typography.Paragraph className="text-white text-xs max-w-64">
                    {t('Neither the sound nor the transcription leaves your computer')}
                </Typography.Paragraph>
            </TooltipContent>
        </Tooltip>
    );
};
