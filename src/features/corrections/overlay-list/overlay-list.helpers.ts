import { EntryView, ListOverlay } from '../corrections.types';

/** Turns a bundled entry on or off. */
export const setEntryEnabled = <T>(overlay: ListOverlay<T>, key: string, enabled: boolean): ListOverlay<T> => {
    const others = overlay.disabled.filter((disabled) => disabled !== key);
    return { ...overlay, disabled: enabled ? others : [...others, key] };
};

/** Deletes an entry of the user, or forgets a disabled entry the data no longer has. */
export const removeEntry = <T>(overlay: ListOverlay<T>, view: EntryView<T>): ListOverlay<T> => {
    if (view.origin === 'orphaned') {
        return setEntryEnabled(overlay, view.key, true);
    }
    const removed = JSON.stringify(view.entry);
    return { ...overlay, added: overlay.added.filter((entry) => JSON.stringify(entry) !== removed) };
};
