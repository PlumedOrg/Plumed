import clsx from 'clsx';
import { useTranslation } from '@/i18n';
import { cleanUpOptionTiming, pipelineStepTitle, stepNumber } from '@/features/corrections/corrections.helpers';
import { StepBadge } from '@/features/corrections/step-badge/step-badge';
import {
    DiffPart,
    HistoryStep,
    HistoryStepId,
    StepChange,
    changesInStepOrder,
    wordDiff,
} from './history-steps.helpers';

interface HistoryStepsProps {
    steps: HistoryStep[];
}

const DIFF_CLASSES: Record<DiffPart['kind'], string> = {
    same: 'text-zinc-300',
    removed: 'text-red-400 line-through',
    added: 'text-green-400',
};

const Diff = ({ parts }: { parts: DiffPart[] }) => (
    <span>
        {parts.map((part, index) => (
            <span key={index} className={DIFF_CLASSES[part.kind]}>
                {index > 0 && ' '}
                {part.text}
            </span>
        ))}
    </span>
);

/** Details of a history entry: one line per processing step, with what it changed. */
export const HistorySteps = ({ steps }: HistoryStepsProps) => {
    const { t } = useTranslation();

    // Recognition carries the badges of steps 1 and 2; a Clean-up option is a
    // sub-line of step 4 (no badge, indented) saying when it really ran.
    const stepLabel = (id: HistoryStepId) => {
        if (id === 'recognition') {
            return (
                <>
                    <StepBadge step={stepNumber('vocabulary')} />
                    <StepBadge step={stepNumber('sound_alikes')} />
                    {t('Recognition')}
                </>
            );
        }
        const timing = cleanUpOptionTiming(id, t);
        if (timing !== null) {
            return (
                <span className="pl-7">
                    {pipelineStepTitle(id, t)} <span className="text-muted-foreground">({timing})</span>
                </span>
            );
        }
        return (
            <>
                <StepBadge step={stepNumber(id)} />
                {pipelineStepTitle(id, t)}
            </>
        );
    };

    const outcome = ({ step, before }: StepChange) => {
        if (before == null) {
            return <span className="text-zinc-300">{step.text_after}</span>;
        }
        if (before === step.text_after) {
            return <span className="italic text-muted-foreground">{t('Unchanged')}</span>;
        }
        const parts = wordDiff(before, step.text_after);
        if (parts.every((part) => part.kind === 'same')) {
            return <span className="italic text-muted-foreground">{t('Spaces adjusted')}</span>;
        }
        return <Diff parts={parts} />;
    };

    return (
        <ul className="space-y-1 border-t border-surface-border pt-2 text-xs" data-testid="history-steps">
            {changesInStepOrder(steps).map((change) => (
                <li key={change.step.step_id} className="grid grid-cols-[14rem_1fr] gap-2">
                    <span
                        className={clsx(
                            'flex items-start gap-1.5 text-muted-foreground',
                            change.before != null && change.before !== change.step.text_after && 'text-sky-300'
                        )}
                    >
                        {stepLabel(change.step.step_id)}
                    </span>
                    {outcome(change)}
                </li>
            ))}
        </ul>
    );
};
