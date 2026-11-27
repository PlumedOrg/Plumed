import { AudioLines } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { BooleanSettingItem } from '../boolean-setting-item/boolean-setting-item';

export const ShortDictationContextSettings = () => {
    const { t } = useTranslation();
    return (
        <BooleanSettingItem
            settingKey="short_dictation_context"
            command="set_short_dictation_context"
            errorMessage={t('Failed to save the short dictation setting')}
            icon={AudioLines}
            title={t('Context for short dictations')}
            description={t(
                'A very short dictation (under 2 seconds or 3 words or fewer, often a corrected word) is transcribed again behind the end of the previous dictation, which improves recognition. That audio stays in memory only, never on disk, and is forgotten after 5 minutes.'
            )}
        />
    );
};
