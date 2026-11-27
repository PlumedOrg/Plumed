import { Switch } from '@/components/switch';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { BooleanSettingKey, useBooleanSetting } from '@/features/settings/hooks/use-boolean-setting';

type BackupPart = 'audio' | 'text' | 'metadata' | 'persistent';

const PARTS: { part: BackupPart; label: string }[] = [
    { part: 'audio', label: 'Audio' },
    { part: 'text', label: 'Transcript' },
    { part: 'metadata', label: 'Progress' },
    { part: 'persistent', label: 'Keep it in the app folder' },
];

const BackupSwitch = ({ part, label }: { part: BackupPart; label: string }) => {
    const { t } = useTranslation();
    const [enabled, setEnabled] = useBooleanSetting(
        `meeting_backup_${part}` as BooleanSettingKey,
        'set_meeting_backup',
        t('Failed to save the meeting backup setting'),
        { part }
    );
    return (
        <label className="flex items-center gap-2">
            <Switch checked={enabled} onCheckedChange={setEnabled} data-testid={`meeting-backup-${part}`} />
            {t(label)}
        </label>
    );
};

// Plumed fork (written with Claude Code): what the meeting backup saves (see
// src-tauri/src/meeting/backup.rs). Off by default; read when a meeting starts.
export const MeetingBackupSettings = () => {
    const { t } = useTranslation();
    return (
        <div className="space-y-2 text-sm">
            <div className="flex flex-wrap items-center gap-3">
                <span>{t('Backup')}</span>
                {PARTS.map(({ part, label }) => (
                    <BackupSwitch key={part} part={part} label={label} />
                ))}
            </div>
            <Typography.Paragraph className="text-xs text-muted-foreground">
                {t(
                    'Saves the meeting as it goes, to resume it after a crash. In the temporary folder unless kept in the app folder. Applies to the next meeting; New meeting deletes it.'
                )}
            </Typography.Paragraph>
        </div>
    );
};
