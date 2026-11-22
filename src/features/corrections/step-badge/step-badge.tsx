import clsx from 'clsx';

interface StepBadgeProps {
    /** Number of the step in the Explanation flow (see `PIPELINE_STEPS`). */
    step: number;
    className?: string;
}

/**
 * A step number in a small filled circle, in the theme accent color. Shared by
 * the Corrections tabs, the Explanation cards, the Clean-up tab headings and
 * the history step lines, so a step looks the same everywhere.
 */
export const StepBadge = ({ step, className }: StepBadgeProps) => (
    <span
        className={clsx(
            'inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-sky-600 text-[11px] font-semibold leading-none text-white tabular-nums',
            className
        )}
        data-testid={`step-badge-${step}`}
    >
        {step}
    </span>
);
