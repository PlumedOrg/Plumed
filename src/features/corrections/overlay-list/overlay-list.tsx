import React, { useState } from 'react';
import { Pencil, Trash2 } from 'lucide-react';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { SettingsUI } from '@/components/settings-ui';
import { Switch } from '@/components/switch';
import { useTranslation } from '@/i18n';
import { useDebouncedInvoke } from '@/features/personalize/medical-rules/hooks/use-debounced-invoke';
import { EntryView, ListOverlay, ListPage } from '../corrections.types';
import { PAGE_SIZE, Pagination } from '../pagination/pagination';
import { removeEntry, setEntryEnabled } from './overlay-list.helpers';

interface OverlayListProps<T> {
    /** Tauri command returning a `ListPage<T>` for `{ query, offset, limit }`. */
    searchCommand: string;
    overlay: ListOverlay<T>;
    saveOverlay: (overlay: ListOverlay<T>) => Promise<boolean>;
    renderEntry: (entry: T) => React.ReactNode;
    searchPlaceholder: string;
    title: string;
    testId: string;
    /** Makes the entries editable, bundled ones included: called with the entry the user wants to change. */
    onEdit?: (view: EntryView<T>) => void;
    /** The form to add an entry, under the list. */
    children: React.ReactNode;
}

/** A bundled list with the user's changes: search, a page of entries, switches, and an add form. */
export const OverlayList = <T,>({
    searchCommand,
    overlay,
    saveOverlay,
    renderEntry,
    searchPlaceholder,
    title,
    testId,
    onEdit,
    children,
}: OverlayListProps<T>) => {
    const { t } = useTranslation();
    const [query, setQuery] = useState('');
    const [offset, setOffset] = useState(0);
    const emptyPage: ListPage<T> = { total: 0, entries: [] };
    const page = useDebouncedInvoke(searchCommand, { query, offset, limit: PAGE_SIZE }, emptyPage, overlay);

    const search = (text: string) => {
        setQuery(text);
        setOffset(0);
    };

    return (
        <SettingsUI.Section
            title={title}
            titleTestId={`${testId}-title`}
            aside={<span className="text-xs text-muted-foreground">{page.total.toLocaleString()}</span>}
        >
            <div className="space-y-4 p-4">
                <Input
                    value={query}
                    onChange={(e) => search(e.target.value)}
                    placeholder={searchPlaceholder}
                    data-testid={`${testId}-search`}
                />
                <div className="divide-y divide-surface-border" data-testid={`${testId}-entries`}>
                    {page.entries.map((view) => (
                        <div key={`${view.origin}-${view.key}`} className="flex items-center gap-2 py-1.5">
                            <div className="flex-1 text-sm">{renderEntry(view.entry)}</div>
                            {view.origin === 'added' && (
                                <span className="text-xs italic text-muted-foreground">{t('Added by you')}</span>
                            )}
                            {view.origin === 'orphaned' && (
                                <span className="text-xs italic text-muted-foreground">
                                    {t('No longer in the medical rules')}
                                </span>
                            )}
                            {onEdit != null && view.origin !== 'orphaned' && (
                                <Button
                                    variant="subtle"
                                    size="icon-sm"
                                    aria-label={t('Edit')}
                                    onClick={() => onEdit(view)}
                                >
                                    <Pencil />
                                </Button>
                            )}
                            {view.origin === 'bundled' ? (
                                <Switch
                                    checked={view.enabled}
                                    onCheckedChange={(enabled) =>
                                        saveOverlay(setEntryEnabled(overlay, view.key, enabled))
                                    }
                                    aria-label={t('Enabled')}
                                />
                            ) : (
                                <Button
                                    variant="subtle"
                                    size="icon-sm"
                                    aria-label={t('Delete')}
                                    onClick={() => saveOverlay(removeEntry(overlay, view))}
                                >
                                    <Trash2 />
                                </Button>
                            )}
                        </div>
                    ))}
                    {page.entries.length === 0 && (
                        <p className="py-2 text-sm text-muted-foreground">{t('No results')}</p>
                    )}
                </div>
                <Pagination offset={offset} total={page.total} onOffsetChange={setOffset} />
                {children}
            </div>
        </SettingsUI.Section>
    );
};
