import { TextStepId, stepNumber } from '@/features/corrections/corrections.helpers';

/** Recognition covers steps 1 and 2, the other ids are the text steps after it. */
export type HistoryStepId = 'recognition' | Exclude<TextStepId, 'vocabulary' | 'sound_alikes'>;

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

/** Words and runs of whitespace, so a change of spacing is a visible change: "a  b" gives ["a", "  ", "b"]. */
const tokens = (text: string): string[] => text.match(/\s+|\S+/g) ?? [];

/** Merges consecutive tokens of the same kind: [same "le", same " ", same "patient"] becomes [same "le patient"]. */
const pushPart = (parts: DiffPart[], kind: DiffKind, token: string) => {
    const last: DiffPart | undefined = parts[parts.length - 1];
    if (last !== undefined && last.kind === kind) {
        last.text += token;
    } else {
        parts.push({ kind, text: token });
    }
};

/**
 * Word level diff (longest common subsequence) that also tracks whitespace,
 * the parts joined back give the texts. Example:
 * wordDiff("le le patient", "le patient") gives
 * [{ kind: 'same', text: 'le ' }, { kind: 'removed', text: 'le ' }, { kind: 'same', text: 'patient' }].
 */
export const wordDiff = (before: string, after: string): DiffPart[] => {
    const a = tokens(before);
    const b = tokens(after);
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

const VISIBLE_WHITESPACE: Record<string, string> = { ' ': '·', '\n': '↵', '\t': '→' };

/**
 * Text to show for a part. A removed or added part made only of whitespace
 * would be invisible, so its characters are drawn: "  " becomes "··".
 */
export const partDisplayText = (part: DiffPart): string =>
    part.kind !== 'same' && part.text.trim().length === 0
        ? Array.from(part.text, (char) => VISIBLE_WHITESPACE[char] ?? '·').join('')
        : part.text;

export interface StepChange {
    step: HistoryStep;
    /** The text before this step, null for recognition (the first text). */
    before: string | null;
}

/** Pairs each step with the text it started from. */
export const stepChanges = (steps: HistoryStep[]): StepChange[] =>
    steps.map((step, index) => ({ step, before: index === 0 ? null : steps[index - 1].text_after }));

/** Step number a history line is shown under: 0 for recognition (steps 1 and 2), first. */
const displayStep = (id: HistoryStepId): number => (id === 'recognition' ? 0 : stepNumber(id));

/**
 * The changes in Explanation step order. The code runs short text correction
 * after step 5 and the pre-installed formatting after step 6, but both are
 * options of step 4, so they are shown under it (the sort is stable, so they
 * follow the clean-up line in the order they ran). Each keeps the text it
 * really started from, so its diff stays right. Example, as step ids:
 * [recognition, dictionary, clean_up, model_corrections, short_text,
 * formatting_rules, built_in_formatting] is shown as [recognition,
 * dictionary, clean_up, short_text, built_in_formatting, model_corrections,
 * formatting_rules].
 */
export const changesInStepOrder = (steps: HistoryStep[]): StepChange[] =>
    [...stepChanges(steps)].sort((a, b) => displayStep(a.step.step_id) - displayStep(b.step.step_id));
