import { Separator } from '@radix-ui/react-separator';
import clsx from 'clsx';
import { ChevronDown, ChevronRight, type LucideIcon } from 'lucide-react';
import React from 'react';

interface SectionProps extends React.HTMLAttributes<HTMLDivElement> {
    title: string;
    icon?: LucideIcon;
    iconColor?: string;
    titleTestId?: string;
    aside?: React.ReactNode;
    /** With `onToggle`, the title folds and unfolds the content. */
    collapsed?: boolean;
    onToggle?: () => void;
}

export const SettingsUI = {
    Container: ({ children, className, ...props }: React.HTMLAttributes<HTMLDivElement>) => {
        return (
            <div className={clsx('border border-surface-border bg-surface rounded-md w-full', className)} {...props}>
                {children}
            </div>
        );
    },

    Section: ({
        title,
        icon: Icon,
        iconColor,
        titleTestId,
        aside,
        collapsed = false,
        onToggle,
        children,
        className,
        ...props
    }: SectionProps) => {
        const Chevron = collapsed ? ChevronRight : ChevronDown;
        return (
            <div className={clsx('border border-surface-border bg-surface rounded-md w-full', className)} {...props}>
                <div
                    className={clsx(
                        'flex items-center gap-2 px-4 py-4 border-surface-border bg-surface-header',
                        collapsed ? 'rounded-[5px]' : 'border-b rounded-t-[5px]'
                    )}
                >
                    {Icon && (
                        <Icon
                            className={clsx('w-5 h-5', iconColor == null && 'text-sky-400')}
                            style={{ color: iconColor }}
                        />
                    )}
                    {onToggle == null ? (
                        <span data-testid={titleTestId} className="font-medium text-base text-sky-400">
                            {title}
                        </span>
                    ) : (
                        <button
                            type="button"
                            onClick={onToggle}
                            aria-expanded={!collapsed}
                            data-testid={titleTestId}
                            className="flex cursor-pointer items-center gap-1 font-medium text-base text-sky-400"
                        >
                            <Chevron className="w-4 h-4" />
                            {title}
                        </button>
                    )}
                    {aside != null && <div className="ml-auto">{aside}</div>}
                </div>
                {!collapsed && <div>{children}</div>}
            </div>
        );
    },

    Item: ({ children, className, ...props }: React.HTMLAttributes<HTMLDivElement>) => {
        return (
            <div className={clsx('p-4 justify-between items-center flex flex-row gap-8', className)} {...props}>
                {children}
            </div>
        );
    },

    Description: ({ children, className, ...props }: React.HTMLAttributes<HTMLDivElement>) => {
        return (
            <div className={clsx('w-96 space-y-2', className)} {...props}>
                {children}
            </div>
        );
    },

    Separator: ({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) => {
        return <Separator className={clsx('border-t border-surface-border', className)} {...props} />;
    },
};
