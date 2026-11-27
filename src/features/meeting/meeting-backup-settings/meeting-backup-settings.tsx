import { useEffect, useState } from 'react';
import { ChevronDown, Settings2 } from 'lucide-react';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/collapsible';
import { Switch } from '@/components/switch';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { BooleanSettingKey, useBooleanSetting } from '@/features/settings/hooks/use-boolean-setting';

type BackupPart = 'audio' | 'text' | 'metadata' | 'persistent';

const PARTS: { part: BackupPart; label: string; description: string }[] = [
    {
        part: 'audio',
        label: 'Back up the audio',
        description:
            'The recording itself (about 115 MB per hour). When resuming, Plumed uses it to recognize the voices heard before the crash.',
    },
    {
        part: 'text',
        label: 'Back up the transcript',
        description: 'The lines written so far, given back after a crash instead of transcribing the audio again.',
    },
    {
        part: 'metadata',
        label: 'Back up the meeting information',
        description:
            'When the meeting started, how much of it was transcribed and when it was last saved, shown when Plumed offers to resume it.',
    },
    {
        part: 'persistent',
        label: 'Keep the backup in the app folder',
        description:
            'Otherwise it goes to the temporary folder, which the system may empty (at restart for example). Either way, New meeting deletes it.',
    },
];

const BackupSwitch = ({ part, label, description }: (typeof PARTS)[number]) => {
    const { t } = useTranslation();
    const [enabled, setEnabled] = useBooleanSetting(
        `meeting_backup_${part}` as BooleanSettingKey,
        'set_meeting_backup',
        t('Failed to save the meeting backup setting'),
        { part }
    );
    return (
        <label className="flex items-start justify-between gap-6 py-2">
            <span className="space-y-0.5">
                <span className="block text-sm">{t(label)}</span>
                <span className="block text-xs text-muted-foreground">{t(description)}</span>
            </span>
            <Switch checked={enabled} onCheckedChange={setEnabled} data-testid={`meeting-backup-${part}`} />
        </label>
    );
};

// Plumed fork (written with Claude Code): the settings of the Réunion page,
// for now what the meeting backup saves (see src-tauri/src/meeting/backup.rs;
// off by default, read when a meeting starts). Open when the page opens so
// the user sees them, closed once the meeting runs.
export const MeetingBackupSettings = ({ running }: { running: boolean }) => {
    const { t } = useTranslation();
    const [open, setOpen] = useState(!running);

    useEffect(() => {
        if (running) setOpen(false);
    }, [running]);

    return (
        <Collapsible
            open={open}
            onOpenChange={setOpen}
            className="rounded-md border border-border bg-background"
            data-testid="meeting-settings"
        >
            <CollapsibleTrigger className="flex w-full cursor-pointer items-center gap-2 p-4 text-sm font-medium">
                <Settings2 className="h-4 w-4 text-muted-foreground" />
                {t('Meeting settings')}
                <ChevronDown className={`ml-auto h-4 w-4 transition-transform ${open ? 'rotate-180' : ''}`} />
            </CollapsibleTrigger>
            <CollapsibleContent className="px-4 pb-4">
                <Typography.Paragraph className="text-xs text-muted-foreground">
                    {t(
                        'Backup: Plumed can save the meeting as it goes, to resume it after a crash or a power cut. Applies to the next meeting.'
                    )}
                </Typography.Paragraph>
                <div className="divide-y divide-border">
                    {PARTS.map((part) => (
                        <BackupSwitch key={part.part} {...part} />
                    ))}
                </div>
            </CollapsibleContent>
        </Collapsible>
    );
};
