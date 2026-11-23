import { MedicalDataCheckReport, MedicalDataSetError, MedicalDataStatus } from './types';

type Translate = (key: string, options?: Record<string, string>) => string;

/** "Medical rules version: ..." line, naming the downloaded version when one is in use. */
export const medicalDataVersionLabel = (status: MedicalDataStatus, t: Translate): string =>
    status.downloaded_version == null
        ? t('Medical rules version: bundled {{bundled}}', { bundled: status.bundled_version })
        : t('Medical rules version: downloaded {{downloaded}} (bundled {{bundled}})', {
              downloaded: status.downloaded_version,
              bundled: status.bundled_version,
          });

/** "Last check: <date>, <result>" line of the settings. */
export const medicalDataLastCheckLabel = (
    report: MedicalDataCheckReport | null,
    t: Translate,
    language: string
): string => {
    if (report == null) {
        return t('No check since Plumed started.');
    }
    const date = new Date(report.checked_at).toLocaleString(language);
    switch (report.outcome) {
        case 'up_to_date':
            return t('Last check: {{date}}, up to date.', { date });
        case 'update_found':
            return t('Last check: {{date}}, version {{version}} available.', { date, version: report.version });
        case 'failed':
            return t('Last check: {{date}}, failed ({{error}}).', { date, error: report.error });
    }
};

const isSetError = (error: unknown): error is MedicalDataSetError =>
    typeof error === 'object' && error != null && 'kind' in error;

/** Why downloaded files were refused, in the user's language. */
export const medicalDataSetErrorMessage = (error: unknown, t: Translate): string => {
    if (!isSetError(error)) {
        return String(error);
    }
    switch (error.kind) {
        case 'missing_file':
            return t('The file {{name}} is missing from the chosen files.', { name: error.name });
        case 'bad_signature':
            return t(
                'The signature does not match: these files were not published by the Plumed maintainer, or were changed.'
            );
        case 'invalid_manifest':
            return t('The manifest is not valid: {{detail}}', { detail: error.detail });
        case 'wrong_model':
            return t('These files are for the model {{model}}, Plumed uses {{expected}}.', {
                model: error.model,
                expected: error.expected,
            });
        case 'app_too_old':
            return t('These files need Plumed {{version}} or newer, update Plumed first.', {
                version: error.min_app_version,
            });
        case 'not_newer':
            return t('Version {{version}} is not newer than the data in use ({{active}}).', {
                version: error.version,
                active: error.active,
            });
        case 'file_mismatch':
            return t('The file {{name}} does not match the manifest (changed, or incomplete download).', {
                name: error.name,
            });
        case 'other':
            return error.detail;
    }
};
