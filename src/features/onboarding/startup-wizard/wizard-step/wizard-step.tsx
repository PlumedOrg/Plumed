import { ReactNode } from 'react';
import { LucideIcon } from 'lucide-react';

interface WizardStepProps {
    icon: LucideIcon;
    title: string;
    description?: string;
    children?: ReactNode;
    footer: ReactNode;
}

// Plumed fork (written with Claude Code): the layout shared by every startup
// wizard step: icon and title, an optional description, the content and the
// action buttons. The content scrolls so a tall step never leaves the screen.
export const WizardStep = ({ icon: Icon, title, description, children, footer }: WizardStepProps) => (
    <div className="space-y-4">
        <header className="space-y-3">
            <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-sky-500/15 rounded-full flex items-center justify-center flex-shrink-0">
                    <Icon className="w-4 h-4 text-sky-400" />
                </div>
                <h2 className="text-lg font-semibold leading-tight tracking-tight">{title}</h2>
            </div>
            {description !== undefined && <p className="text-sm text-muted-foreground">{description}</p>}
        </header>
        {children !== undefined && <div className="max-h-[50vh] space-y-4 overflow-y-auto pr-1">{children}</div>}
        <footer className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">{footer}</footer>
    </div>
);
