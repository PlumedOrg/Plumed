import clsx from 'clsx';
import { Button } from '@/components/button';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { PreviewArrow } from '@/features/extensions/extension-demo/preview-arrow/preview-arrow';
import { PreviewCell } from '@/features/extensions/extension-demo/preview-cell/preview-cell';
import { StepBadge } from '../../step-badge/step-badge';

export interface StepStatus {
    label: string;
    active: boolean;
}

export interface StepAction {
    label: string;
    onClick: () => void;
    testId: string;
}

/** A concrete text before and after the step. */
export interface StepExample {
    before: string;
    after: string;
}

export interface Step {
    title: string;
    /** When the step runs: while decoding, on the audio again, or on the text. */
    stage: string;
    description: string;
    examples: StepExample[];
    note?: string;
    statuses: StepStatus[];
    actions: StepAction[];
}

interface PipelineStepProps {
    index: number;
    step: Step;
}

/** One card of the Explanation tab flow: what the step does, a before and after example, its state and buttons. */
export const PipelineStep = ({ index, step }: PipelineStepProps) => {
    const { t } = useTranslation();

    return (
        <section
            className="space-y-3 rounded-md border border-surface-border bg-surface-header p-4"
            data-testid={`corrections-step-${index}`}
        >
            <div className="flex flex-wrap items-center gap-2">
                <StepBadge step={index} />
                <Typography.Title>{step.title}</Typography.Title>
                <span className="text-xs text-muted-foreground">{step.stage}</span>
                {step.statuses.map((status) => (
                    <span
                        key={status.label}
                        className={clsx(
                            'rounded-full px-2 py-0.5 text-xs',
                            status.active ? 'bg-sky-500/15 text-sky-300' : 'bg-muted text-muted-foreground'
                        )}
                    >
                        {status.label}
                    </span>
                ))}
            </div>
            <Typography.Paragraph>{step.description}</Typography.Paragraph>
            {step.examples.map((example) => (
                <div key={example.before} className="grid grid-cols-1 gap-2 text-left md:grid-cols-[1fr_auto_1fr]">
                    <PreviewCell label={t('Before')} centered>
                        <span className="text-sm italic text-foreground">{example.before}</span>
                    </PreviewCell>
                    <PreviewArrow horizontalFrom="md" />
                    <PreviewCell label={t('After')} centered>
                        <span className="text-sm text-foreground">{example.after}</span>
                    </PreviewCell>
                </div>
            ))}
            {step.note != null && <p className="text-xs italic text-muted-foreground">{step.note}</p>}
            {step.actions.length > 0 && (
                <div className="flex flex-wrap gap-2">
                    {step.actions.map((action) => (
                        <Button
                            key={action.testId}
                            variant="secondary"
                            size="sm"
                            onClick={action.onClick}
                            data-testid={action.testId}
                        >
                            {action.label}
                        </Button>
                    ))}
                </div>
            )}
        </section>
    );
};
