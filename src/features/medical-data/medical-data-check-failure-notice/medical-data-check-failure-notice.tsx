import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { ExternalLink } from '@/components/external-link';
import { PLUMED_MEDICAL_DATA_RELEASE_URL } from '@/utils/plumed-links';

interface CheckFailureBodyProps {
    error: string;
}

const CheckFailureBody = ({ error }: CheckFailureBodyProps) => {
    const { t } = useTranslation();

    return (
        <div className="flex flex-col gap-2 text-sm" data-testid="medical-data-check-failed">
            <p>{t('Plumed could not check for medical rules updates ({{error}}).', { error })}</p>
            <p className="text-xs">
                {t(
                    'Your network may block Plumed but allow your browser: open the release page, download the files, then install them at the bottom of Settings > Corrections > Explanation with "Install a downloaded update".'
                )}
            </p>
            <ExternalLink href={PLUMED_MEDICAL_DATA_RELEASE_URL}>{t('Open the medical data release page')}</ExternalLink>
        </div>
    );
};

// Plumed fork (written with Claude Code): a non-blocking notice when the
// startup medical data check failed (network error, GitHub blocked, bad
// signature...). The backend hands the failure out once per launch, so the
// notice never shows twice, even when the page is opened again.
export const MedicalDataCheckFailureNotice = () => {
    useEffect(() => {
        const show = () => {
            invoke<string | null>('take_medical_data_check_failure')
                .then((error) => {
                    if (error != null) {
                        toast.warning(<CheckFailureBody error={error} />, {
                            toastId: 'medical-data-check-failed',
                            autoClose: false,
                        });
                    }
                })
                .catch((e) => console.error('Failed to read the medical data check result:', e));
        };
        show();
        const unlisten = listen('medical-data-check-failed', show);
        return () => {
            void unlisten.then((stop) => stop());
        };
    }, []);

    return null;
};
