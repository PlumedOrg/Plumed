import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { Textarea } from '@/components/textarea';
import { useTranslation } from '@/i18n';
import { EntryView, ListOverlay, TextTemplate } from '../corrections.types';
import { useListFile } from '../hooks/use-list-file';
import { useListOverlay } from '../hooks/use-list-overlay';
import { ListHeader } from '../list-header/list-header';
import { OverlayList } from '../overlay-list/overlay-list';
import { countTemplateChanges, saveTemplateEdit } from './text-templates.helpers';

const EMPTY_OVERLAY: ListOverlay<TextTemplate> = { disabled: [], added: [] };

const TemplateEntry = ({ template }: { template: TextTemplate }) => (
    <div>
        <div className="font-medium">{template.trigger}</div>
        <div className="line-clamp-3 whitespace-pre-line text-xs text-muted-foreground">{template.text}</div>
    </div>
);

/** "Text templates" tab: a spoken phrase replaced by a longer text. Not a numbered step, runs at the start of step 6. */
export const TextTemplates = () => {
    const { t } = useTranslation();
    const { overlay, saveOverlay, reload } = useListOverlay<TextTemplate>(
        'get_text_template_overlay',
        'set_text_template_overlay'
    );
    const { exportList, importList } = useListFile('templates', reload);
    // The state of a fresh install: bundled templates are off by default.
    const [initial, setInitial] = useState<ListOverlay<TextTemplate>>(EMPTY_OVERLAY);
    const [trigger, setTrigger] = useState('');
    const [text, setText] = useState('');
    const [editing, setEditing] = useState<EntryView<TextTemplate> | null>(null);

    useEffect(() => {
        invoke<ListOverlay<TextTemplate>>('get_initial_text_template_overlay')
            .then(setInitial)
            .catch((error: unknown) => console.error('get_initial_text_template_overlay failed:', error));
    }, []);

    const clearForm = () => {
        setTrigger('');
        setText('');
        setEditing(null);
    };

    const startEdit = (view: EntryView<TextTemplate>) => {
        setEditing(view);
        setTrigger(view.entry.trigger);
        setText(view.entry.text);
    };

    const saveTemplate = async () => {
        const saved = await saveOverlay(saveTemplateEdit(overlay, editing, { trigger: trigger.trim(), text }));
        if (saved) {
            clearForm();
        }
    };

    return (
        <div className="space-y-6">
            <ListHeader
                title={t('Text templates')}
                description={t(
                    'A text template replaces a spoken phrase by a longer text, for example "Insère l\'examen clinique normal." becomes a complete normal clinical exam. Templates are applied at the start of step 6, before your formatting rules, and match like the other spoken commands (capitals and final punctuation do not matter). The example is off until you turn it on.'
                )}
                summary={countTemplateChanges(overlay, initial)}
                resetDescription={t(
                    'Your added templates are deleted and the list is back to its original state, with the example turned off. This cannot be undone.'
                )}
                onReset={() => saveOverlay(initial)}
                onExport={exportList}
                onImport={importList}
                testId="text-templates"
            />
            <OverlayList
                searchCommand="search_text_templates"
                overlay={overlay}
                saveOverlay={saveOverlay}
                renderEntry={(template) => <TemplateEntry template={template} />}
                searchPlaceholder={t('Search a phrase or a text')}
                title={t('Templates')}
                testId="text-templates-list"
                onEdit={startEdit}
            >
                <div className="space-y-2">
                    <Input
                        value={trigger}
                        onChange={(e) => setTrigger(e.target.value)}
                        placeholder={t('What you say')}
                        data-testid="text-templates-new-trigger"
                    />
                    <Textarea
                        value={text}
                        onChange={(e) => setText(e.target.value)}
                        placeholder={t('The text it is replaced by')}
                        rows={6}
                        data-testid="text-templates-new-text"
                    />
                    <div className="flex justify-end gap-2">
                        {editing != null && (
                            <Button variant="subtle" size="sm" onClick={clearForm}>
                                {t('Cancel')}
                            </Button>
                        )}
                        <Button
                            variant="secondary"
                            size="sm"
                            disabled={trigger.trim().length === 0 || text.trim().length === 0}
                            onClick={saveTemplate}
                            data-testid="text-templates-add"
                        >
                            {editing != null ? t('Save') : t('Add')}
                        </Button>
                    </div>
                </div>
            </OverlayList>
        </div>
    );
};
