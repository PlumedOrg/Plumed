import { ListOverlay, TextTemplate } from '../corrections.types';

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
