import clsx from 'clsx';
import React, { useEffect, useId, useRef } from 'react';
import { ChevronLeft, ChevronRight, type LucideIcon } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { TabPanel } from './tab-panel/tab-panel';
import { TabTrigger } from './tab-trigger/tab-trigger';
import { getAdjacentTabIndex } from './tabs.helpers';
import { useSidewaysScroll } from './use-sideways-scroll';

export interface TabItem<T extends string> {
    value: T;
    label: string;
    count?: number;
    icon?: LucideIcon;
    /** Plumed fork: shown before the label, e.g. a step number badge. */
    prefix?: React.ReactNode;
}

interface TabsProps<T extends string> {
    value: T;
    onValueChange: (value: T) => void;
    tabs: TabItem<T>[];
    children: React.ReactNode;
    className?: string;
    activeColor?: string;
}

export const Tabs = <T extends string>({
    value,
    onValueChange,
    tabs,
    children,
    className,
    activeColor,
}: TabsProps<T>) => {
    const { t } = useTranslation();
    const baseId = useId();
    const { rowRef, canScrollLeft, canScrollRight, scrollByStep, rowHandlers } = useSidewaysScroll();
    const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);
    const panelId = `${baseId}-panel`;
    const getTabId = (tabValue: T) => `${baseId}-tab-${tabValue}`;
    const isFirstTabActive = tabs.length > 0 && tabs[0].value === value;
    const activeIndex = tabs.findIndex((tab) => tab.value === value);

    // Plumed fork: the tab row scrolls sideways when it is wider than the
    // page (Corrections has many tabs), so keep the active tab visible.
    useEffect(() => {
        tabRefs.current[activeIndex]?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    }, [activeIndex]);

    const handleKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
        const nextIndex = getAdjacentTabIndex(index, event.key, tabs.length);
        if (nextIndex == null) return;
        event.preventDefault();
        onValueChange(tabs[nextIndex].value);
        tabRefs.current[nextIndex]?.focus();
    };

    return (
        <div className={className}>
            <div className="relative z-10 -mb-px flex items-end">
                {/* Shown only when the tabs overflow: the arrows tell people
                    who would not think of scrolling that more tabs exist. */}
                {(canScrollLeft || canScrollRight) && (
                    <ScrollArrow
                        direction={-1}
                        disabled={!canScrollLeft}
                        label={t('Show the previous tabs')}
                        onClick={scrollByStep}
                    />
                )}
                {/* pb-px/-mb-px: the active tab overlaps the panel border by one
                    pixel, which the scroll container would otherwise clip. The
                    native scrollbar is hidden: the arrows, the wheel and click
                    and drag scroll the row. */}
                <div
                    ref={rowRef}
                    role="tablist"
                    {...rowHandlers}
                    className="no-scrollbar -mb-px flex min-w-0 flex-1 overflow-x-auto overflow-y-hidden pb-px"
                >
                    {tabs.map((tab, index) => {
                        const isActive = tab.value === value;
                        return (
                            <TabTrigger
                                key={tab.value}
                                ref={(element) => {
                                    tabRefs.current[index] = element;
                                }}
                                type="button"
                                role="tab"
                                id={getTabId(tab.value)}
                                aria-selected={isActive}
                                aria-controls={panelId}
                                tabIndex={isActive ? 0 : -1}
                                onClick={() => onValueChange(tab.value)}
                                onKeyDown={(event) => handleKeyDown(event, index)}
                                isActive={isActive}
                                className={clsx(
                                    'shrink-0 cursor-pointer whitespace-nowrap py-3',
                                    isActive && activeColor == null && 'text-sky-400'
                                )}
                                style={isActive ? { color: activeColor } : undefined}
                            >
                                {tab.prefix}
                                {tab.icon != null && <tab.icon className="w-4 h-4" />}
                                {tab.label}
                                {tab.count != null && (
                                    <span
                                        className={clsx(
                                            'min-w-5 rounded-full px-1.5 py-0.5 text-center text-xs leading-none tabular-nums',
                                            isActive ? 'bg-sky-500/15 text-sky-300' : 'bg-muted text-muted-foreground'
                                        )}
                                    >
                                        {tab.count}
                                    </span>
                                )}
                            </TabTrigger>
                        );
                    })}
                </div>
                {(canScrollLeft || canScrollRight) && (
                    <ScrollArrow
                        direction={1}
                        disabled={!canScrollRight}
                        label={t('Show the next tabs')}
                        onClick={scrollByStep}
                    />
                )}
            </div>
            <TabPanel
                role="tabpanel"
                id={panelId}
                aria-labelledby={getTabId(value)}
                isFirstTabActive={isFirstTabActive}
            >
                {children}
            </TabPanel>
        </div>
    );
};

interface ScrollArrowProps {
    direction: -1 | 1;
    disabled: boolean;
    label: string;
    onClick: (direction: -1 | 1) => void;
}

/** Plumed fork: a button beside the tab row that scrolls it one step. */
const ScrollArrow = ({ direction, disabled, label, onClick }: ScrollArrowProps) => {
    const Icon = direction === -1 ? ChevronLeft : ChevronRight;
    return (
        <button
            type="button"
            disabled={disabled}
            aria-label={label}
            title={label}
            onClick={() => onClick(direction)}
            className="mb-px shrink-0 cursor-pointer self-stretch rounded-md px-1.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground disabled:cursor-default disabled:opacity-30 disabled:hover:bg-transparent"
        >
            <Icon className="h-5 w-5" />
        </button>
    );
};
