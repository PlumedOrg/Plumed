import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

const DEBOUNCE_MS = 250;

/**
 * Calls the Tauri `command` with `args` once they stop changing, and returns
 * its latest result. `refreshKey` forces a new call (e.g. after the rules
 * changed). `null` args give `emptyResult` without calling.
 */
export const useDebouncedInvoke = <T>(
    command: string,
    args: Record<string, unknown> | null,
    emptyResult: T,
    refreshKey: unknown = null
): T => {
    const [result, setResult] = useState<T>(emptyResult);
    const argsKey = JSON.stringify(args);

    useEffect(() => {
        if (args == null) {
            setResult(emptyResult);
            return;
        }
        let cancelled = false;
        const timer = setTimeout(() => {
            invoke<T>(command, args)
                .then((value) => {
                    if (!cancelled) {
                        setResult(value);
                    }
                })
                .catch((error: unknown) => console.error(`${command} failed:`, error));
        }, DEBOUNCE_MS);
        return () => {
            cancelled = true;
            clearTimeout(timer);
        };
    }, [command, argsKey, refreshKey]);

    return result;
};

/** The args of a command taking one text, or `null` (no call) while it is blank. */
export const textArgs = (name: string, text: string): Record<string, string> | null =>
    text.trim().length > 0 ? { [name]: text } : null;
