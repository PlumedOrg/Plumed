import { DiffPart, partDisplayText, wordDiff } from './history-steps.helpers';

const DIFF_CLASSES: Record<DiffPart['kind'], string> = {
    same: 'text-zinc-300',
    removed: 'text-red-400 line-through',
    added: 'text-green-400',
};

interface StepDiffProps {
    before: string;
    after: string;
}

/** Colored word diff of what a step changed: removed words in red, added ones in green. Shared by the history and the Corrections Explanation tab. */
export const StepDiff = ({ before, after }: StepDiffProps) => (
    <span className="whitespace-pre-wrap">
        {wordDiff(before, after).map((part, index) => (
            <span key={index} className={DIFF_CLASSES[part.kind]}>
                {partDisplayText(part)}
            </span>
        ))}
    </span>
);
