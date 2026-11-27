import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useNavigate } from '@tanstack/react-router';
import { History, Loader2 } from 'lucide-react';
import { toast } from 'react-toastify';
import { useTranslation } from '@/i18n';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/dialog';
import { Page } from '@/components/page';
import { formatDuration } from '../meeting.helpers';
import { MeetingBackup } from '../meeting.types';

/** Asked once per launch, not each time Home is shown again. */
let asked = false;

// Plumed fork (written with Claude Code): a meeting backup left by a previous
// run (crash, power cut) is either resumed or deleted, nothing else.
export const MeetingBackupPrompt = () => {
    const { t, i18n } = useTranslation();
    const navigate = useNavigate();
    const [backup, setBackup] = useState<MeetingBackup | null>(null);
    const [busy, setBusy] = useState(false);

    useEffect(() => {
        if (asked) {
            return;
        }
        asked = true;
        invoke<MeetingBackup | null>('meeting_backup_find')
            .then(setBackup)
            .catch((e) => console.error('Failed to look for a meeting backup:', e));
    }, []);

    const resume = async () => {
        setBusy(true);
        try {
            await invoke('meeting_backup_resume');
            setBackup(null);
            void navigate({ to: '/meeting' });
        } catch (e) {
            toast.error(`${t('Could not resume the meeting')}: ${String(e)}`);
        } finally {
            setBusy(false);
        }
    };

    const remove = async () => {
        await invoke('meeting_backup_delete').catch((e) => console.error(e));
        setBackup(null);
    };

    const savedAt =
        backup?.updated_at != null ? new Date(backup.updated_at * 1000).toLocaleString(i18n.language) : null;

    return (
        <Dialog open={backup != null}>
            <DialogContent
                className="max-w-lg"
                overlayClassName="backdrop-blur-sm"
                hideCloseButton
                onEscapeKeyDown={(e) => e.preventDefault()}
                onPointerDownOutside={(e) => e.preventDefault()}
                data-testid="meeting-backup-prompt"
            >
                <DialogHeader className="space-y-3">
                    <div className="flex items-center gap-3">
                        <div className="w-8 h-8 bg-sky-500/15 rounded-full flex items-center justify-center flex-shrink-0">
                            <History className="w-4 h-4 text-sky-400" />
                        </div>
                        <DialogTitle>{t('An unfinished meeting was saved')}</DialogTitle>
                    </div>
                    <DialogDescription>
                        {backup?.has_text
                            ? t('{{count}} lines, {{time}} of meeting.', {
                                  count: backup.lines,
                                  time: formatDuration(backup.processed_secs),
                              })
                            : t('{{time}} of audio, transcribed again if you resume.', {
                                  time: formatDuration(backup?.processed_secs ?? 0),
                              })}
                        {savedAt != null && ` ${t('Saved on {{date}}.', { date: savedAt })}`}
                    </DialogDescription>
                </DialogHeader>
                <DialogFooter>
                    <Page.SecondaryButton
                        onClick={() => void remove()}
                        disabled={busy}
                        data-testid="meeting-backup-delete"
                    >
                        {t('Delete')}
                    </Page.SecondaryButton>
                    <Page.PrimaryButton
                        autoFocus
                        onClick={() => void resume()}
                        disabled={busy}
                        data-testid="meeting-backup-resume"
                    >
                        {busy && <Loader2 className="h-4 w-4 animate-spin mr-2" />}
                        {t('Resume')}
                    </Page.PrimaryButton>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};
