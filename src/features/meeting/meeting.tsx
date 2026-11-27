import { Users } from 'lucide-react';
import { ExtensionPageLayout } from '@/features/extensions/extension-page-layout/extension-page-layout';
import { ExtensionHeader } from '@/features/extensions/extension-header/extension-header';
import { useTranslation } from '@/i18n';
import { useMeeting } from './hooks/use-meeting';
import { MeetingBackupSettings } from './meeting-backup-settings/meeting-backup-settings';
import { MeetingControls } from './meeting-controls/meeting-controls';
import { MeetingModelGate } from './meeting-model-gate/meeting-model-gate';
import { MeetingTranscript } from './meeting-transcript/meeting-transcript';

// Plumed fork (written with Claude Code): the Réunion page, a meeting
// transcript with its speakers, from the microphone, PhoneMic or a file.
export const MeetingPage = () => {
    const { t } = useTranslation();
    const meeting = useMeeting();

    return (
        <ExtensionPageLayout colorScheme="standard" isActive>
            <ExtensionHeader
                icon={Users}
                title={t('Meeting')}
                colorScheme="standard"
                titleTestId="meeting-title"
                subtitle={t('Transcribe a meeting, up to 4 hours, and tell its speakers apart. Everything runs on this computer.')}
                badge={t('Experimental')}
            />
            <MeetingModelGate>
                <MeetingControls meeting={meeting} />
                <MeetingBackupSettings />
                <MeetingTranscript lines={meeting.lines} />
            </MeetingModelGate>
        </ExtensionPageLayout>
    );
};
