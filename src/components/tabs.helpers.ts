/** The tab a URL hash names, or `fallback` when it names none of `tabs`. */
export const pickTabFromHash = <T extends string>(hash: string, tabs: readonly T[], fallback: T): T =>
    tabs.find((tab) => tab === hash) ?? fallback;

export const getAdjacentTabIndex = (currentIndex: number, key: string, tabCount: number): number | undefined => {
    switch (key) {
        case 'ArrowRight':
            return (currentIndex + 1) % tabCount;
        case 'ArrowLeft':
            return (currentIndex - 1 + tabCount) % tabCount;
        default:
            return undefined;
    }
};
