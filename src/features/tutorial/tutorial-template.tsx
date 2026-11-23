import { ReactNode } from 'react';
import { Page } from '@/components/page';
import { useTranslation } from '@/i18n';
import { ListPage, TextTemplate } from '@/features/corrections/corrections.types';
import { useListOverlay } from '@/features/corrections/hooks/use-list-overlay';
import { setEntryEnabled } from '@/features/corrections/overlay-list/overlay-list.helpers';
import { useDebouncedInvoke } from '@/features/personalize/medical-rules/hooks/use-debounced-invoke';
import { TEMPLATE_EXAMPLE } from './tutorial.data';

const EMPTY_PAGE: ListPage<TextTemplate> = { total: 0, entries: [] };

/**
 * Plumed fork (written with Claude Code): the bundled text template the
 * tutorial shows. Bundled templates start off, so while neither it nor an
 * edited copy is on, a button turns the bundled one on.
 * `children` shows the spoken example.
 */
export const TutorialTemplate = ({ children }: { children: ReactNode }) => {
    const { t } = useTranslation();
    const { overlay, saveOverlay } = useListOverlay<TextTemplate>(
        'get_text_template_overlay',
        'set_text_template_overlay'
    );
    // Refreshed with the overlay, so the button goes away once it is on.
    const page = useDebouncedInvoke(
        'search_text_templates',
        { query: TEMPLATE_EXAMPLE.say, offset: 0, limit: 10 },
        EMPTY_PAGE,
        overlay
    );
    const bundled = page.entries.find((view) => view.origin === 'bundled');
    const isOn = page.entries.some((view) => view.enabled && view.entry.trigger === TEMPLATE_EXAMPLE.say);

    return (
        <div className="space-y-2">
            {children}
            {bundled !== undefined && !isOn && (
                <div className="flex items-center justify-between gap-3 text-sm text-muted-foreground">
                    <span>{t('This template is off for now.')}</span>
                    <Page.SecondaryButton
                        onClick={() => void saveOverlay(setEntryEnabled(overlay, bundled.key, true))}
                        data-testid="tutorial-template-on"
                    >
                        {t('Turn it on')}
                    </Page.SecondaryButton>
                </div>
            )}
        </div>
    );
};
