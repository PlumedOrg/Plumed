// Plumed fork (written with Claude Code): the startup questions show one per
// launch, in this order: PhoneMic, usage statistics, medical data check, then
// the medical data update offer. Each later one waits for a launch where no
// earlier one is due.
import { invoke } from '@tauri-apps/api/core';
import { AppSettings } from '@/features/settings/settings.types';

/** Whether the PhoneMic or the usage statistics question is asked this launch. */
export const isEarlierStartupPromptDue = async (): Promise<boolean> => {
    const [settings, remoteMicDue, telemetryAvailable] = await Promise.all([
        invoke<AppSettings>('get_all_settings'),
        invoke<boolean>('remote_mic_prompt_due'),
        invoke<boolean>('is_telemetry_available'),
    ]);
    return remoteMicDue || (telemetryAvailable && !settings.telemetry_asked);
};
