import { SettingsUI } from '@/components/settings-ui';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { useModelStatus } from '../hooks/use-model-status';
import { ModelChooser } from '../model-chooser/model-chooser';

// Plumed fork (written with Claude Code): Settings > Model, the
// same catalog as the first launch dialog.
export const ModelSettings = () => {
    const { t } = useTranslation();
    const { status, setStatus } = useModelStatus();

    if (status == null) {
        return null;
    }

    return (
        <SettingsUI.Container className="space-y-3 p-4" data-testid="model-settings">
            <Typography.Title>{t('Speech model')}</Typography.Title>
            <Typography.Paragraph className="text-xs">
                {t(
                    'Models are downloaded from Hugging Face, or from the Plumed relay when Hugging Face cannot be reached, and checked before use. They are kept across updates.'
                )}
            </Typography.Paragraph>
            {status.dev && (
                <Typography.Paragraph className="text-xs" data-testid="model-dev">
                    {t('Development build: the model of resources/ is used while none is downloaded.')}
                </Typography.Paragraph>
            )}
            <ModelChooser status={status} onStatus={setStatus} />
        </SettingsUI.Container>
    );
};
