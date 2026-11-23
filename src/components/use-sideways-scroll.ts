import React, { useCallback, useEffect, useRef, useState } from 'react';

// Plumed fork (written with Claude Code): helpers for a row that scrolls
// sideways (the Corrections tab bar), for people who do not think of the
// mouse wheel or a trackpad: arrow buttons and click and drag.

/** Moves under this many pixels are a click, not a drag. */
const DRAG_THRESHOLD_PX = 5;

/** One arrow click scrolls this share of the visible width. */
const ARROW_STEP = 0.7;

export interface SidewaysScroll {
    rowRef: React.RefObject<HTMLDivElement | null>;
    canScrollLeft: boolean;
    canScrollRight: boolean;
    /** -1 scrolls left, 1 scrolls right. */
    scrollByStep: (direction: -1 | 1) => void;
    /** Spread on the scrolling row. */
    rowHandlers: {
        onScroll: () => void;
        onWheel: (event: React.WheelEvent<HTMLDivElement>) => void;
        onPointerDown: (event: React.PointerEvent<HTMLDivElement>) => void;
        onPointerMove: (event: React.PointerEvent<HTMLDivElement>) => void;
        onPointerUp: (event: React.PointerEvent<HTMLDivElement>) => void;
        onPointerCancel: (event: React.PointerEvent<HTMLDivElement>) => void;
        onClickCapture: (event: React.MouseEvent<HTMLDivElement>) => void;
    };
}

export const useSidewaysScroll = (): SidewaysScroll => {
    const rowRef = useRef<HTMLDivElement | null>(null);
    const [canScrollLeft, setCanScrollLeft] = useState(false);
    const [canScrollRight, setCanScrollRight] = useState(false);
    // Where the press started, null when no button is pressed.
    const drag = useRef<{ startX: number; startScroll: number; moved: boolean } | null>(null);
    // Set when a drag ends, so the click that follows does not switch tab.
    const swallowClick = useRef(false);

    // Example: scrollWidth 900, clientWidth 600, scrollLeft 0 gives
    // left false, right true. One pixel of slack absorbs rounding.
    const update = useCallback(() => {
        const row = rowRef.current;
        if (row == null) return;
        setCanScrollLeft(row.scrollLeft > 1);
        setCanScrollRight(row.scrollLeft + row.clientWidth < row.scrollWidth - 1);
    }, []);

    useEffect(() => {
        const row = rowRef.current;
        if (row == null) return;
        update();
        const observer = new ResizeObserver(update);
        observer.observe(row);
        // The tabs inside change width when their labels or counts change.
        Array.from(row.children).forEach((child) => observer.observe(child));
        return () => observer.disconnect();
    }, [update]);

    const scrollByStep = (direction: -1 | 1) => {
        const row = rowRef.current;
        row?.scrollBy({ left: direction * row.clientWidth * ARROW_STEP, behavior: 'smooth' });
    };

    // A plain mouse wheel only scrolls vertically: turn it into a sideways
    // scroll while the row overflows, so mouse users can reach every tab.
    const onWheel = (event: React.WheelEvent<HTMLDivElement>) => {
        const row = event.currentTarget;
        const overflows = row.scrollWidth > row.clientWidth;
        if (!overflows || event.deltaY === 0 || Math.abs(event.deltaX) > Math.abs(event.deltaY)) return;
        row.scrollLeft += event.deltaY;
    };

    const onPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
        // Touch and pens already scroll natively.
        if (event.pointerType !== 'mouse' || event.button !== 0) return;
        // A drag whose click never came must not eat this one.
        swallowClick.current = false;
        drag.current = { startX: event.clientX, startScroll: event.currentTarget.scrollLeft, moved: false };
    };

    const onPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
        const state = drag.current;
        if (state == null) return;
        const dx = event.clientX - state.startX;
        if (!state.moved) {
            if (Math.abs(dx) < DRAG_THRESHOLD_PX) return;
            state.moved = true;
            // Keeps the drag going when the pointer leaves the row.
            event.currentTarget.setPointerCapture(event.pointerId);
        }
        event.currentTarget.scrollLeft = state.startScroll - dx;
    };

    const endDrag = (event: React.PointerEvent<HTMLDivElement>) => {
        if (drag.current?.moved === true) {
            swallowClick.current = true;
            event.currentTarget.releasePointerCapture(event.pointerId);
        }
        drag.current = null;
    };

    const onClickCapture = (event: React.MouseEvent<HTMLDivElement>) => {
        if (!swallowClick.current) return;
        swallowClick.current = false;
        event.stopPropagation();
        event.preventDefault();
    };

    return {
        rowRef,
        canScrollLeft,
        canScrollRight,
        scrollByStep,
        rowHandlers: {
            onScroll: update,
            onWheel,
            onPointerDown,
            onPointerMove,
            onPointerUp: endDrag,
            onPointerCancel: endDrag,
            onClickCapture,
        },
    };
};
