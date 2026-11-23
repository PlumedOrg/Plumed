import clsx from 'clsx';
import { useTranslation } from '@/i18n';
import { cleanUpOptionTiming, stepName, stepNumber } from '@/features/corrections/corrections.helpers';
import { StepBadge } from '@/features/corrections/step-badge/step-badge';
import { HistoryStep, HistoryStepId, StepChange, changesInStepOrder } from './history-steps.helpers';
import { StepDiff } from './step-diff';

interface HistoryStepsProps {
    steps: HistoryStep[];
}

/** Details of a history entry: one line per processing step, with what it changed. */
export const HistorySteps = ({ steps }: HistoryStepsProps) => {
    const { t } = useTranslation();

    // Recognition carries steps 1 and 2 (the engine applies the medical
    // vocabulary and the sound-alike words while it decodes), so it shows both
    // badges and names; a Clean-up option is a sub-line of step 4 (no badge,
    // indented) saying when it really ran.
    const stepLabel = (id: HistoryStepId) => {
        if (id === 'recognition') {
            return (
                <>
                    <StepBadge step={stepNumber('vocabulary')} />
                    <StepBadge step={stepNumber('sound_alikes')} />
                    {stepName('vocabulary', t)}, {stepName('sound_alikes', t)}
                </>
            );
        }
        const timing = cleanUpOptionTiming(id, t);
        if (timing !== null) {
            return (
                <span className="pl-7">
                    {stepName(id, t)} <span className="text-muted-foreground">({timing})</span>
                </span>
            );
        }
        return (
            <>
                <StepBadge step={stepNumber(id)} />
                {stepName(id, t)}
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
        return <StepDiff before={before} after={step.text_after} />;
    };

    return (
        <ul
            className="space-y-3 border-t border-surface-border pt-3 text-xs leading-relaxed"
            data-testid="history-steps"
        >
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
