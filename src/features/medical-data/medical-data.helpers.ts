import { MedicalDataStatus } from './types';

type Translate = (key: string, options?: Record<string, string>) => string;

/** "Medical data version: ..." line, naming the downloaded version when one is in use. */
export const medicalDataVersionLabel = (status: MedicalDataStatus, t: Translate): string =>
    status.downloaded_version == null
        ? t('Medical data version: bundled {{bundled}}', { bundled: status.bundled_version })
        : t('Medical data version: downloaded {{downloaded}} (bundled {{bundled}})', {
              downloaded: status.downloaded_version,
              bundled: status.bundled_version,
          });
