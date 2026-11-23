import { useEffect, useRef, useState } from 'react';
import { Channel, invoke } from '@tauri-apps/api/core';
import { relaunch } from '@tauri-apps/plugin-process';
import { RefreshCcw, Download } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { PLUMED_LATEST_RELEASE_URL } from '@/utils/plumed-links';
import { useAppUpdateChannel } from './hooks/use-app-update-channel';

type UpdateCheckerProps = {
    className?: string;
};

// Plumed fork (written with Claude Code): the check and the install run in
// Rust (`src-tauri/src/app_update.rs`), which picks the channel's manifest and
// falls back to the Plumed relay when GitHub is unreachable. The button shows
// only when an update is available, or always when the user disabled the
// automatic check (then it checks the stable channel on demand).
export const UpdateChecker = ({ className = '' }: UpdateCheckerProps) => {
    const [channel] = useAppUpdateChannel();
    const [isChecking, setIsChecking] = useState(false);
    const [updateAvailable, setUpdateAvailable] = useState(false);
    const [isInstalling, setIsInstalling] = useState(false);
    const [downloadProgress, setDownloadProgress] = useState(0);
    const [showUpToDate, setShowUpToDate] = useState(false);
    const [isPacmanManaged, setIsPacmanManaged] = useState(false);
    const { t } = useTranslation();

    const upToDateTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

    useEffect(() => {
        invoke<boolean>('is_pacman_managed')
            .then(setIsPacmanManaged)
            .catch((e) => {
                console.error('Failed to check pacman install:', e);
            });
        return () => {
            if (upToDateTimeoutRef.current) clearTimeout(upToDateTimeoutRef.current);
        };
    }, []);

    // Checks again whenever the channel changes: a beta found earlier is not
    // an update on the stable channel.
    useEffect(() => {
        setUpdateAvailable(false);
        if (channel !== null && channel !== 'disabled') void checkForUpdates(false);
    }, [channel]);

    const checkForUpdates = async (manual: boolean) => {
        try {
            setIsChecking(true);
            const version = await invoke<string | null>('check_app_update', { manual });
            const found = version !== null;
            setUpdateAvailable(found);
            setShowUpToDate(manual && !found);
            if (manual && !found) {
                if (upToDateTimeoutRef.current) clearTimeout(upToDateTimeoutRef.current);
                upToDateTimeoutRef.current = setTimeout(() => setShowUpToDate(false), 3000);
            }
        } catch (e) {
            console.error('Failed to check for updates:', e);
        } finally {
            setIsChecking(false);
        }
    };

    const installUpdate = async () => {
        try {
            setIsInstalling(true);
            setDownloadProgress(0);
            const onProgress = new Channel<number>();
            onProgress.onmessage = setDownloadProgress;
            await invoke('install_app_update', { onProgress });
            try {
                await relaunch();
            } catch (e) {
                console.error('Failed to relaunch after install:', e);
            }
        } catch (e) {
            console.error('Failed to install update:', e);
        } finally {
            setIsInstalling(false);
            setDownloadProgress(0);
        }
    };

    const getUpdateStatusText = () => {
        if (isInstalling) {
            if (downloadProgress > 0 && downloadProgress < 100)
                return t('Downloading... {{progress}}%', {
                    progress: String(downloadProgress).padStart(3),
                });
            if (downloadProgress === 100) return t('Installing...');
            return t('Preparing...');
        }
        if (isChecking) return t('Checking...');
        if (showUpToDate) return t('Up to date');
        if (updateAvailable) return t('Update available');
        return t('Check for updates');
    };

    const onClick = () => {
        if (updateAvailable && !isInstalling) return installUpdate();
        if (!isChecking && !isInstalling && !updateAvailable) return checkForUpdates(true);
    };

    if (!updateAvailable && !isInstalling && channel !== 'disabled') return null;

    const isDisabled = isChecking || isInstalling;
    const isClickable = !isDisabled && (updateAvailable || !showUpToDate);

    if (isPacmanManaged && updateAvailable && !isInstalling) {
        return (
            <a
                href={PLUMED_LATEST_RELEASE_URL}
                target="_blank"
                rel="noopener noreferrer"
                className={`text-xs transition-colors flex items-center gap-1.5 px-2 py-1 rounded cursor-pointer text-sky-500 hover:text-sky-400 hover:bg-sky-500/10 ${className}`}
            >
                <Download className="w-4 h-4" />
                <span>{t('Update available')}</span>
            </a>
        );
    }

    return (
        <button
            onClick={onClick}
            disabled={isDisabled || !isClickable}
            className={`text-xs transition-colors flex items-center gap-1.5 px-2 py-1 rounded cursor-pointer disabled:opacity-50 ${
                updateAvailable && !isInstalling
                    ? 'text-sky-500 hover:text-sky-400 hover:bg-sky-500/10'
                    : 'text-muted-foreground hover:text-foreground hover:bg-accent'
            } ${className}`}
        >
            {updateAvailable && !isInstalling ? <Download className="w-4 h-4" /> : <RefreshCcw className="w-4 h-4" />}
            <span>{getUpdateStatusText()}</span>
        </button>
    );
};

export default UpdateChecker;
