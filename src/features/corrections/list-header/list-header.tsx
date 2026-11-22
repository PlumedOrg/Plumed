import { useState } from 'react';
import { Download, Undo2, Upload } from 'lucide-react';
import { Button } from '@/components/button';
import {
    Dialog,
    DialogClose,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/dialog';
import { Page } from '@/components/page';
import { SettingsUI } from '@/components/settings-ui';
import { Switch } from '@/components/switch';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';

interface ListOption {
    checked: boolean;
    onChange: (checked: boolean) => void;
    testId: string;
}

interface ListHeaderProps {
    title: string;
    description: string;
    /** The on/off option of the step that uses this list. */
    option?: ListOption;
    /**
     * Number of the user's changes to a bundled list (Reset is off without
     * any), or the status of a list of the user's own (Reset always on).
     */
    summary: number | string;
    /** What Reset brings back, for the confirmation dialog. */
    resetDescription: string;
    onReset: () => void;
    onExport: () => void;
    onImport: () => void;
    testId: string;
}

/** Top of a list tab of the Corrections page: what the list does, its option, and Import, Export, Reset. */
export const ListHeader = ({
    title,
    description,
    option,
    summary,
    resetDescription,
    onReset,
    onExport,
    onImport,
    testId,
}: ListHeaderProps) => {
    const { t } = useTranslation();
    const [confirmOpen, setConfirmOpen] = useState(false);
    const canReset = typeof summary === 'string' || summary > 0;
    const summaryLabel =
        typeof summary === 'string'
            ? summary
            : summary > 0
              ? t('{{count}} change(s) of yours', { count: summary })
              : t('No change of yours');

    const reset = () => {
        onReset();
        setConfirmOpen(false);
    };

    return (
        <SettingsUI.Container data-testid={testId}>
            <SettingsUI.Item>
                <SettingsUI.Description className="w-150">
                    <Typography.Title>{title}</Typography.Title>
                    <Typography.Paragraph>{description}</Typography.Paragraph>
                </SettingsUI.Description>
                {option != null && (
                    <Switch checked={option.checked} onCheckedChange={option.onChange} data-testid={option.testId} />
                )}
            </SettingsUI.Item>
            <SettingsUI.Separator />
            <div className="flex flex-wrap items-center gap-1 px-3 py-1.5">
                <span className="mr-auto text-xs text-muted-foreground" data-testid={`${testId}-changes`}>
                    {summaryLabel}
                </span>
                <Button variant="subtle" size="sm" onClick={onImport} data-testid={`${testId}-import`}>
                    <Download />
                    {t('Import')}
                </Button>
                <Button variant="subtle" size="sm" onClick={onExport} data-testid={`${testId}-export`}>
                    <Upload />
                    {t('Export')}
                </Button>
                <Button
                    variant="subtle"
                    size="sm"
                    disabled={!canReset}
                    onClick={() => setConfirmOpen(true)}
                    data-testid={`${testId}-reset`}
                >
                    <Undo2 />
                    {t('Back to the original list')}
                </Button>
            </div>
            <Dialog open={confirmOpen} onOpenChange={setConfirmOpen}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>{t('Back to the original list')}</DialogTitle>
                        <DialogDescription>{resetDescription}</DialogDescription>
                    </DialogHeader>
                    <DialogFooter>
                        <DialogClose asChild>
                            <Page.SecondaryButton>{t('Cancel')}</Page.SecondaryButton>
                        </DialogClose>
                        <Button variant="destructive" onClick={reset} data-testid={`${testId}-reset-confirm`}>
                            {t('Delete my changes')}
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </SettingsUI.Container>
    );
};
