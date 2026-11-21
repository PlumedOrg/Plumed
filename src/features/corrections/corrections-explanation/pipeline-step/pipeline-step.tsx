import clsx from 'clsx';
import React from 'react';
import { Button } from '@/components/button';
import { Typography } from '@/components/typography';

export interface StepStatus {
    label: string;
    active: boolean;
}

export interface StepAction {
    label: string;
    onClick: () => void;
    testId: string;
}

interface PipelineStepProps {
    index: number;
    title: string;
    /** When the step runs: while decoding, on the audio again, or on the text. */
    stage: string;
    statuses: StepStatus[];
    actions: StepAction[];
    isLast: boolean;
    children: React.ReactNode;
}

/** One numbered step of the vertical flow of the Explanation tab. */
export const PipelineStep = ({ index, title, stage, statuses, actions, isLast, children }: PipelineStepProps) => {
    return (
        <li className="flex gap-4" data-testid={`corrections-step-${index}`}>
            <div className="flex flex-col items-center">
                <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-sky-500/15 text-sm font-semibold text-sky-300">
                    {index}
                </span>
                {!isLast && <span className="mt-1 w-px flex-1 bg-surface-border" />}
            </div>
            <div className="flex-1 space-y-2 pb-6">
                <div className="flex flex-wrap items-center gap-2">
                    <Typography.Title>{title}</Typography.Title>
                    <span className="text-xs text-muted-foreground">{stage}</span>
                    {statuses.map((status) => (
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
                {children}
                {actions.length > 0 && (
                    <div className="flex flex-wrap gap-2">
                        {actions.map((action) => (
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
            </div>
        </li>
    );
};
