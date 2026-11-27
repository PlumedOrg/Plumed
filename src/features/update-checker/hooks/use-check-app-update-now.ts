import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { useAppUpdate } from './use-app-update';

// Plumed fork (written with Claude Code): a check on demand that tells the
// result in a toast, for Settings > Online and the sidebar version number.
// A found update is installed from the sidebar button, which the Rust check
// announces (`app_update::check`).
export const useCheckAppUpdateNow = () => {
    const { t } = useTranslation();
    const { isChecking, checkForUpdates } = useAppUpdate();

    const checkNow = async () => {
        if (isChecking) return;
        if (await checkForUpdates(true))
            toast.info(t('A new version is available: install it from the sidebar button.'));
        else toast.info(t('Plumed is up to date.'));
    };

    return { isChecking, checkNow };
};
