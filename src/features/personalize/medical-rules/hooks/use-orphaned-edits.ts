import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { MedicalRuleEdit } from '../types';

/** Patterns of the saved edits whose rule the medical data in use no longer has (after a data update). */
export const useOrphanedEdits = (edits: MedicalRuleEdit[]) => {
    const [orphaned, setOrphaned] = useState<Set<string>>(new Set());

    useEffect(() => {
        invoke<string[]>('get_orphaned_medical_rule_edits')
            .then((patterns) => setOrphaned(new Set(patterns)))
            .catch((error: unknown) => console.error('Failed to check medical rule edits:', error));
    }, [edits]);

    return orphaned;
};
