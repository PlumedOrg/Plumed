import { MedicalRuleEdit, MedicalRuleStatus } from './types';

/** How the "Your changes" list shows an edit; `orphaned` holds the patterns of rules that no longer exist. */
export const editStatus = (edit: MedicalRuleEdit, orphaned: Set<string>): MedicalRuleStatus => {
    if (edit.original == null) {
        return 'added';
    }
    return orphaned.has(edit.original) ? 'orphaned' : 'changed';
};

/** The edit of a bundled rule, if the user changed it. */
export const findEdit = (edits: MedicalRuleEdit[], original: string): MedicalRuleEdit | undefined =>
    edits.find((edit) => edit.original === original);

/** Replaces the edit of the same bundled rule, or appends a new one. */
export const upsertEdit = (edits: MedicalRuleEdit[], edit: MedicalRuleEdit): MedicalRuleEdit[] => {
    if (edit.original == null) {
        return [...edits, edit];
    }
    const index = edits.findIndex((other) => other.original === edit.original);
    if (index === -1) {
        return [...edits, edit];
    }
    return edits.map((other, i) => (i === index ? edit : other));
};

export const replaceEditAt = (edits: MedicalRuleEdit[], index: number, edit: MedicalRuleEdit): MedicalRuleEdit[] =>
    edits.map((other, i) => (i === index ? edit : other));

export const removeEditAt = (edits: MedicalRuleEdit[], index: number): MedicalRuleEdit[] =>
    edits.filter((_, i) => i !== index);
