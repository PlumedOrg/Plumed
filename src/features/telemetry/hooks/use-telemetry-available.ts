import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

// Plumed fork: usage statistics exist only in builds made with a Umami
// website id, other builds never mention them.
export const useTelemetryAvailable = () => {
    const [available, setAvailable] = useState(false);

    useEffect(() => {
        invoke<boolean>('is_telemetry_available')
            .then(setAvailable)
            .catch((e) => console.error('Failed to check whether usage statistics are available:', e));
    }, []);

    return available;
};
