import { useSyncExternalStore } from 'react';
import type { HistoryEntry } from '@/features/home/history/hooks/use-history-state';
import { ALL_EXAMPLES, SpokenCommand } from './tutorial.data';

// Plumed fork (written with Claude Code): which tutorial examples the user has
// already dictated, so the tutorial can show the remaining ones in purple.
// UI-only progress, kept in the webview's localStorage (never exported).

const STORAGE_KEY = 'plumed:tutorial-used';
const CHANGED_EVENT = 'plumed:tutorial-used-changed';

/**
 * Lower case, no accents, words separated by single spaces and padded with
 * one, so a phrase is found as whole words:
 * foldForMatch("Point-virgule, à la ligne.") gives " point virgule a la ligne ".
 */
export const foldForMatch = (text: string): string =>
    ` ${text
        .normalize('NFD')
        .replace(/[\u0300-\u036f]/g, '')
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, ' ')
        .trim()} `;

/** Identifies an example in the stored progress. */
export const commandId = (command: SpokenCommand): string => command.say;

/**
 * Whether a dictation used `command`: its spoken phrase (or `heard`, the part
 * without numbers) is in the recognized text, as whole words.
 * Example: { say: 'virgule' } is used by "Le patient virgule calme", not by "virgules".
 */
export const isCommandUsed = (command: SpokenCommand, recognized: string): boolean =>
    foldForMatch(recognized).includes(foldForMatch(command.heard ?? command.say));

const readUsed = (): string[] => {
    try {
        const parsed: unknown = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? '[]');
        return Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === 'string') : [];
    } catch {
        return [];
    }
};

/** Adds `ids` to the used examples, telling the open tutorial when something changed. */
export const markUsed = (ids: string[]) => {
    const used = readUsed();
    const added = ids.filter((id) => !used.includes(id));
    if (added.length === 0) {
        return;
    }
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify([...used, ...added]));
    window.dispatchEvent(new Event(CHANGED_EVENT));
};

/**
 * Marks the examples dictated in `entries` as used. Spoken commands are looked
 * for in the recognized text (before the rules replaced them), or in the final
 * text for entries without steps.
 */
export const markDictatedExamples = (entries: HistoryEntry[]) => {
    const recognized = entries.map(
        (entry) => entry.steps?.find((step) => step.step_id === 'recognition')?.text_after ?? entry.text
    );
    markUsed(ALL_EXAMPLES.filter((command) => recognized.some((text) => isCommandUsed(command, text))).map(commandId));
};

// useSyncExternalStore needs a stable snapshot: the raw string, parsed by the hook.
const snapshot = () => window.localStorage.getItem(STORAGE_KEY) ?? '[]';
const subscribe = (onChange: () => void) => {
    window.addEventListener(CHANGED_EVENT, onChange);
    return () => window.removeEventListener(CHANGED_EVENT, onChange);
};

/** The ids of the examples already dictated, updated live. */
export const useUsedCommands = (): Set<string> => {
    const raw = useSyncExternalStore(subscribe, snapshot);
    return new Set(raw === '[]' ? [] : readUsed());
};
