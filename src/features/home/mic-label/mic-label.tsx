import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { useMicState } from '@/features/settings/system/mic-settings/hooks/use-mic-state';

export const MicLabel = () => {
    const { t } = useTranslation();
    const { currentMic, micList } = useMicState();
    const selectedMic = currentMic === 'automatic' ? null : micList.find((m) => m.id === currentMic);

    // Plumed fork: nothing to say when the default microphone is used
    if (!selectedMic) {
        return null;
    }

    return (
        <Typography.Paragraph className="text-xs absolute top-2 right-2 text-muted-foreground">
            {t('Recording with ')}
            <span className="text-sky-400">{selectedMic.label}</span>
        </Typography.Paragraph>
    );
};
