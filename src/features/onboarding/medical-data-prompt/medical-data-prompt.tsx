import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { MedicalDataUpdateDialog } from '@/features/medical-data/medical-data-update-dialog/medical-data-update-dialog';
import { MedicalDataOffer } from '@/features/medical-data/types';
import { isEarlierStartupPromptDue } from '../startup-prompts.helpers';

// Plumed fork (written with Claude Code): when the startup check (backend,
// after the model loaded) found a newer signed medical data set, asks
// whether to download it. Waits for the next launch when another startup
// question is asked, so dialogs never stack.
export const MedicalDataPrompt = () => {
    const [offer, setOffer] = useState<MedicalDataOffer | null>(null);
    const blocked = useRef<Promise<boolean> | null>(null);

    useEffect(() => {
        blocked.current = isEarlierStartupPromptDue().catch((e) => {
            console.error('Failed to check the other startup questions:', e);
            return true;
        });
        const show = async (found: MedicalDataOffer | null) => {
            if (found == null || blocked.current == null) {
                return;
            }
            if (await blocked.current) {
                // Not this launch, even once the other question is answered.
                invoke('dismiss_medical_data_offer').catch((e) => console.error(e));
                return;
            }
            setOffer(found);
        };
        invoke<MedicalDataOffer | null>('get_medical_data_offer')
            .then(show)
            .catch((e) => console.error('Failed to check for a medical data update:', e));
        const unlisten = listen<MedicalDataOffer>('medical-data-update-available', (event) => void show(event.payload));
        return () => {
            void unlisten.then((stop) => stop());
        };
    }, []);

    const close = () => {
        setOffer(null);
        invoke('dismiss_medical_data_offer').catch((e) => console.error(e));
    };

    return <MedicalDataUpdateDialog offer={offer} onClose={close} />;
};
