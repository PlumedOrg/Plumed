import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { MedicalDataStatus } from '../types';

/** Which medical rules is in use (bundled or downloaded version), `null` while loading. */
export const useMedicalDataStatus = () => {
    const [status, setStatus] = useState<MedicalDataStatus | null>(null);

    const refresh = useCallback(() => {
        invoke<MedicalDataStatus>('get_medical_data_status')
            .then(setStatus)
            .catch((e) => console.error('Failed to load the medical rules status:', e));
    }, []);

    useEffect(refresh, [refresh]);

    return { status, refresh };
};
