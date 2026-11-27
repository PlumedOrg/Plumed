import { Snail } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { BooleanSettingItem } from '../boolean-setting-item/boolean-setting-item';

// Plumed fork (written with Claude Code): experimental, for fast speakers.
export const SlowDownAudioSettings = () => {
    const { t } = useTranslation();
    return (
        <BooleanSettingItem
            settingKey="slow_down_audio"
            command="set_slow_down_audio"
            errorMessage={t('Failed to save the slow down setting')}
            icon={Snail}
            title={t('Slow down the audio (experimental)')}
            description={t(
                'Slows your voice down by 25% (pitch unchanged) before transcribing it, which may help if you speak fast. Transcription takes a little longer. The live preview is not affected.'
            )}
        />
    );
};
