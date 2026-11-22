import { useState } from 'react';

// UI-only preference, kept in the webview's localStorage like the dictionary
// revamp flag (never exported, survives restarts).
const STORAGE_KEY = 'plumed:user-rules-collapsed';

const readCollapsed = () => window.localStorage.getItem(STORAGE_KEY) === 'true';

/** Whether the list of the user's formatting rules is folded. Unfolded by default. */
export const useUserRulesCollapsed = () => {
    const [collapsed, setCollapsed] = useState<boolean>(readCollapsed);

    const save = (value: boolean) => {
        window.localStorage.setItem(STORAGE_KEY, String(value));
        setCollapsed(value);
    };

    return [collapsed, save] as const;
};
