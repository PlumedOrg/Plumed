import clsx from 'clsx';
import React, { useEffect, useId, useRef } from 'react';
import { type LucideIcon } from 'lucide-react';
import { TabPanel } from './tab-panel/tab-panel';
import { TabTrigger } from './tab-trigger/tab-trigger';
import { getAdjacentTabIndex } from './tabs.helpers';

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
    const baseId = useId();
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

    // A plain mouse wheel only scrolls vertically: turn it into a sideways
    // scroll while the row overflows, so mouse users can reach every tab.
    const handleWheel = (event: React.WheelEvent<HTMLDivElement>) => {
        const row = event.currentTarget;
        const overflows = row.scrollWidth > row.clientWidth;
        if (!overflows || event.deltaY === 0 || Math.abs(event.deltaX) > Math.abs(event.deltaY)) return;
        row.scrollLeft += event.deltaY;
    };

    const handleKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
        const nextIndex = getAdjacentTabIndex(index, event.key, tabs.length);
        if (nextIndex == null) return;
        event.preventDefault();
        onValueChange(tabs[nextIndex].value);
        tabRefs.current[nextIndex]?.focus();
    };

    return (
        <div className={className}>
            {/* pb-px/-mb-px: the active tab overlaps the panel border by one
                pixel, which the scroll container would otherwise clip. */}
            <div
                role="tablist"
                onWheel={handleWheel}
                className="relative z-10 -mb-px flex overflow-x-auto overflow-y-hidden pb-px [scrollbar-width:thin]"
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
                            className={clsx('shrink-0 cursor-pointer whitespace-nowrap', isActive && activeColor == null && 'text-sky-400')}
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
