import { useCallback } from 'react';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { MedicalDataChange } from '../types';

/** Tells the user which medical data is now in use, and about edits of rules that are gone. */
export const useMedicalDataChangeToast = () => {
    const { t } = useTranslation();

    return useCallback(
        (change: MedicalDataChange) => {
            toast.success(t('Medical data {{version}} is now in use.', { version: change.version }));
            if (change.orphaned_edits > 0) {
                toast.warning(
                    t(
                        '{{count}} of your edits refer to rules that no longer exist. They are kept, see the drug and medical term rules.',
                        { count: change.orphaned_edits }
                    )
                );
            }
        },
        [t]
    );
};
