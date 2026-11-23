import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/select';
import { useTranslation } from '@/i18n';
import { AppUpdateChannel } from '@/features/settings/settings.types';
import { useAppUpdateChannel } from '../hooks/use-app-update-channel';

const CHANNELS: { value: AppUpdateChannel; label: string }[] = [
    { value: 'stable', label: 'Stable versions' },
    { value: 'beta', label: 'Beta versions' },
    { value: 'disabled', label: 'Disabled' },
];

// Plumed fork (written with Claude Code): the update channel picker, shared by
// Settings > Online and the startup wizard. Saving it also marks the question
// as answered (see `set_app_update_channel`).
export const AppUpdateChannelSelect = () => {
    const { t } = useTranslation();
    const [channel, setChannel] = useAppUpdateChannel();

    return (
        <Select value={channel ?? undefined} onValueChange={(value) => void setChannel(value as AppUpdateChannel)}>
            <SelectTrigger className="w-[180px]" data-testid="app-update-channel-select">
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                {CHANNELS.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                        {t(option.label)}
                    </SelectItem>
                ))}
            </SelectContent>
        </Select>
    );
};
