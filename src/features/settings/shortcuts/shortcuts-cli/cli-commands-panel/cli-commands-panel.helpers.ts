import { PLUMED_DOCS_URL } from '@/utils/plumed-links';

export interface CliCommandDescriptor {
    id: string;
    label: string;
    command: string;
}

// New CLI flags must be added here AND in the Rust `CliCommand` enum and tauri.conf.json.
export const CLI_COMMANDS: CliCommandDescriptor[] = [
    { id: 'transcription', label: 'Toggle transcription', command: 'plumed --transcription' },
    { id: 'paste-last', label: 'Paste last transcript', command: 'plumed --paste-last' },
    { id: 'cancel', label: 'Cancel recording', command: 'plumed --cancel' },
];

/** The Wayland section of the dictation doc: how to bind the commands in the desktop settings. */
export const CLI_DOC_URL = `${PLUMED_DOCS_URL}/fonctionnalites/dictee/#sous-linux-avec-wayland`;
