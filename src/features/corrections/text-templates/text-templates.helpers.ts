import { EntryView, ListOverlay, TextTemplate } from '../corrections.types';

/**
 * Saves `template` in place of `editing` (null when adding). Editing a
 * bundled template turns it off and adds the edited copy as the user's own,
 * so the original stays available by turning it back on.
 */
export const saveTemplateEdit = (
    overlay: ListOverlay<TextTemplate>,
    editing: EntryView<TextTemplate> | null,
    template: TextTemplate
): ListOverlay<TextTemplate> => {
    if (editing?.origin === 'bundled') {
        const disabled = overlay.disabled.filter((key) => key !== editing.key);
        return { disabled: [...disabled, editing.key], added: [...overlay.added, template] };
    }
    const previous = JSON.stringify(editing?.entry ?? null);
    const kept = overlay.added.filter((entry) => JSON.stringify(entry) !== previous);
    return { ...overlay, added: [...kept, template] };
};

/**
 * The user's changes to the templates: added ones, plus the bundled ones whose
 * state differs from a fresh install (where they all start off).
 */
export const countTemplateChanges = (
    overlay: ListOverlay<TextTemplate>,
    initial: ListOverlay<TextTemplate>
): number => {
    const now = new Set(overlay.disabled);
    const start = new Set(initial.disabled);
    const turnedOn = [...start].filter((key) => !now.has(key)).length;
    const turnedOff = [...now].filter((key) => !start.has(key)).length;
    return overlay.added.length + turnedOn + turnedOff;
};
