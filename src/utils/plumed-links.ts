/** Single source of truth for Plumed's public project URLs. */
export const PLUMED_REPO_URL = 'https://github.com/PlumedOrg/Plumed';
export const PLUMED_RELEASES_URL = `${PLUMED_REPO_URL}/releases`;
export const PLUMED_LATEST_RELEASE_URL = `${PLUMED_RELEASES_URL}/latest`;
/** The signed medical data sets (rules, sound-alike groups, vocabulary), see `src-tauri/medical-data/SOURCE.md`. */
export const PLUMED_MEDICAL_DATA_RELEASE_URL = `${PLUMED_RELEASES_URL}/tag/medical-data`;
export const PLUMED_NEW_ISSUE_URL = `${PLUMED_REPO_URL}/issues/new`;
/** Plumed's phone microphone relay, built in (same default as `AppSettings::remote_mic_url` in Rust). */
export const PLUMED_RELAY_URL = 'https://turn.plumed.fr';
/** French health agency page explaining HDS (certified health data hosting). */
export const HDS_INFO_URL = 'https://esante.gouv.fr/produits-services/hds';
