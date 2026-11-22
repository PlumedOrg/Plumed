import { ChevronLeft, ChevronRight } from 'lucide-react';
import { Button } from '@/components/button';
import { useTranslation } from '@/i18n';

export const PAGE_SIZE = 50;

interface PaginationProps {
    /** Index of the first shown item. */
    offset: number;
    total: number;
    onOffsetChange: (offset: number) => void;
}

/** "51-100 of 77,000" with previous and next buttons, for long lists shown a page at a time. */
export const Pagination = ({ offset, total, onOffsetChange }: PaginationProps) => {
    const { t } = useTranslation();
    if (total <= PAGE_SIZE) {
        return null;
    }
    const last = Math.min(offset + PAGE_SIZE, total);

    return (
        <div className="flex items-center justify-end gap-2 text-xs text-muted-foreground">
            <span>
                {t('{{first}}-{{last}} of {{total}}', {
                    first: (offset + 1).toLocaleString(),
                    last: last.toLocaleString(),
                    total: total.toLocaleString(),
                })}
            </span>
            <Button
                variant="subtle"
                size="icon-sm"
                aria-label={t('Previous page')}
                disabled={offset === 0}
                onClick={() => onOffsetChange(Math.max(0, offset - PAGE_SIZE))}
            >
                <ChevronLeft />
            </Button>
            <Button
                variant="subtle"
                size="icon-sm"
                aria-label={t('Next page')}
                disabled={last >= total}
                onClick={() => onOffsetChange(offset + PAGE_SIZE)}
            >
                <ChevronRight />
            </Button>
        </div>
    );
};
