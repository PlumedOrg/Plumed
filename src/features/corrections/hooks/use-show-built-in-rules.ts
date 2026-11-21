import { useState } from 'react';

// UI-only preference, kept in the webview's localStorage like the dictionary
// revamp flag (never exported, survives restarts).
const STORAGE_KEY = 'plumed:show-built-in-rules';

const readShowBuiltIn = () => window.localStorage.getItem(STORAGE_KEY) === 'true';

/** Whether the Rules tab also lists the built-in rules and options. Off by default. */
export const useShowBuiltInRules = () => {
    const [showBuiltIn, setShowBuiltIn] = useState<boolean>(readShowBuiltIn);

    const save = (show: boolean) => {
        window.localStorage.setItem(STORAGE_KEY, String(show));
        setShowBuiltIn(show);
    };

    return [showBuiltIn, save] as const;
};
