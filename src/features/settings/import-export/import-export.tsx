import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { ExportSection } from './export-section/export-section';
import { ImportSection } from './import-section/import-section';

// Plumed fork: body of the last tab of the Settings page (it used to be a page of its own).
export const ImportExport = () => {
    const { t } = useTranslation();

    return (
        <div className="space-y-4">
            <Typography.Paragraph className="text-muted-foreground">
                {t('Save or restore your Murmure configuration.')}
            </Typography.Paragraph>

            <ExportSection />
            <div className="border-t border-border" />
            <ImportSection />
        </div>
    );
};
