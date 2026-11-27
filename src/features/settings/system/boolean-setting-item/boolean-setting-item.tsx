import type { LucideIcon } from 'lucide-react';
import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { Switch } from '@/components/switch';
import { BooleanSettingKey, useBooleanSetting } from '@/features/settings/hooks/use-boolean-setting';

// Plumed fork (written with Claude Code): one settings row with a switch for a
// boolean setting saved through its Tauri command.
interface BooleanSettingItemProps {
    settingKey: BooleanSettingKey;
    command: string;
    errorMessage: string;
    icon: LucideIcon;
    title: string;
    description: string;
}

export const BooleanSettingItem = ({
    settingKey,
    command,
    errorMessage,
    icon: Icon,
    title,
    description,
}: BooleanSettingItemProps) => {
    const [enabled, setEnabled] = useBooleanSetting(settingKey, command, errorMessage);

    return (
        <SettingsUI.Item>
            <SettingsUI.Description>
                <Typography.Title className="flex items-center gap-2">
                    <Icon className="w-4 h-4 text-muted-foreground" />
                    {title}
                </Typography.Title>
                <Typography.Paragraph>{description}</Typography.Paragraph>
            </SettingsUI.Description>
            <Switch checked={enabled} onCheckedChange={setEnabled} />
        </SettingsUI.Item>
    );
};
