import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { GrammarRules } from '../grammar-tab.types';

const EMPTY: GrammarRules = { grammalecte_version: '', rules: [] };

/** The grammar step's rules with their state, and how to change them. */
export const useGrammarRules = () => {
    const { t } = useTranslation();
    const [data, setData] = useState<GrammarRules>(EMPTY);

    const reload = useCallback(() => {
        invoke<GrammarRules>('get_grammar_rules')
            .then(setData)
            .catch((error: unknown) => console.error('get_grammar_rules failed:', error));
    }, []);

    useEffect(reload, [reload]);

    const run = useCallback(
        async (command: string, args?: Record<string, unknown>) => {
            try {
                await invoke(command, args);
            } catch (error) {
                toast.error(`${t('Failed to save the grammar rules')}: ${String(error)}`);
            }
            reload();
        },
        [reload, t]
    );

    const saveEnabled = useCallback((enabled: string[]) => run('set_grammar_rules', { enabled }), [run]);
    const reset = useCallback(() => run('reset_grammar_rules'), [run]);

    return { data, reload, saveEnabled, reset };
};
