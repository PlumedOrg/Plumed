import { PipelineStepId } from '@/features/corrections/corrections.helpers';

/** Recognition covers steps 1 and 2, the other ids are the text steps after it. */
export type HistoryStepId = 'recognition' | Exclude<PipelineStepId, 'vocabulary' | 'sound_alikes'>;

/** The text after one processing step (Rust `history::StepText`). */
export interface HistoryStep {
    step_id: HistoryStepId;
    text_after: string;
}

export type DiffKind = 'same' | 'removed' | 'added';

export interface DiffPart {
    kind: DiffKind;
    text: string;
}

/** Above this many word pairs, the diff shows the whole texts as removed and added. */
const MAX_DIFF_CELLS = 4_000_000;

const words = (text: string): string[] => text.split(/\s+/).filter((word) => word.length > 0);

/** Merges consecutive words of the same kind: [same "le", same "patient"] becomes [same "le patient"]. */
const pushPart = (parts: DiffPart[], kind: DiffKind, word: string) => {
    const last: DiffPart | undefined = parts[parts.length - 1];
    if (last !== undefined && last.kind === kind) {
        last.text = `${last.text} ${word}`;
    } else {
        parts.push({ kind, text: word });
    }
};

/**
 * Word level diff (longest common subsequence). Example:
 * wordDiff("le le patient", "le patient") gives
 * [{ kind: 'same', text: 'le' }, { kind: 'removed', text: 'le' }, { kind: 'same', text: 'patient' }].
 */
export const wordDiff = (before: string, after: string): DiffPart[] => {
    const a = words(before);
    const b = words(after);
    const parts: DiffPart[] = [];
    if (a.length * b.length > MAX_DIFF_CELLS) {
        a.forEach((word) => pushPart(parts, 'removed', word));
        b.forEach((word) => pushPart(parts, 'added', word));
        return parts;
    }
    // lcs[i][j]: length of the longest common subsequence of a[i..] and b[j..].
    const lcs = Array.from({ length: a.length + 1 }, () => new Array<number>(b.length + 1).fill(0));
    for (let i = a.length - 1; i >= 0; i--) {
        for (let j = b.length - 1; j >= 0; j--) {
            lcs[i][j] = a[i] === b[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
        }
    }
    let i = 0;
    let j = 0;
    while (i < a.length && j < b.length) {
        if (a[i] === b[j]) {
            pushPart(parts, 'same', a[i]);
            i++;
            j++;
        } else if (lcs[i + 1][j] >= lcs[i][j + 1]) {
            pushPart(parts, 'removed', a[i]);
            i++;
        } else {
            pushPart(parts, 'added', b[j]);
            j++;
        }
    }
    a.slice(i).forEach((word) => pushPart(parts, 'removed', word));
    b.slice(j).forEach((word) => pushPart(parts, 'added', word));
    return parts;
};

export interface StepChange {
    step: HistoryStep;
    /** The text before this step, null for recognition (the first text). */
    before: string | null;
}

/** Pairs each step with the text it started from. */
export const stepChanges = (steps: HistoryStep[]): StepChange[] =>
    steps.map((step, index) => ({ step, before: index === 0 ? null : steps[index - 1].text_after }));
