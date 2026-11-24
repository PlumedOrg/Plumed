import { useEffect, useRef, useState } from 'react';
import { Channel, invoke } from '@tauri-apps/api/core';
import { relaunch } from '@tauri-apps/plugin-process';

// Plumed fork (written with Claude Code): checking for and installing an app
// update (the work runs in Rust, `src-tauri/src/app_update.rs`), shared by the
// sidebar button and PhoneMic's "update Plumed" offer.
export const useAppUpdate = () => {
    const [isChecking, setIsChecking] = useState(false);
    const [updateAvailable, setUpdateAvailable] = useState(false);
    const [isInstalling, setIsInstalling] = useState(false);
    const [downloadProgress, setDownloadProgress] = useState(0);
    const [showUpToDate, setShowUpToDate] = useState(false);
    const upToDateTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

    useEffect(
        () => () => {
            if (upToDateTimeoutRef.current) clearTimeout(upToDateTimeoutRef.current);
        },
        []
    );

    /** `manual` checks even when the automatic check is disabled. Resolves to whether an update was found. */
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
            return found;
        } catch (e) {
            console.error('Failed to check for updates:', e);
            return false;
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

    return {
        isChecking,
        updateAvailable,
        setUpdateAvailable,
        isInstalling,
        downloadProgress,
        showUpToDate,
        checkForUpdates,
        installUpdate,
    };
};
