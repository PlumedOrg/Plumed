import { Home, Settings, Info, Bug, RadioTower, SpellCheck, GraduationCap, Users } from 'lucide-react';
import { Link, useLocation } from '@tanstack/react-router';
import {
    Sidebar,
    SidebarHeader,
    SidebarContent,
    SidebarFooter,
    SidebarGroup,
    SidebarMenu,
    SidebarMenuItem,
    SidebarMenuButton,
} from '../../../components/sidebar';
import { useGetVersion } from '../hooks/use-get-version';
import { useCheckAppUpdateNow } from '@/features/update-checker/hooks/use-check-app-update-now';
import { UpdateChecker } from '../../update-checker/update-checker';
import { Separator } from '@/components/separator';
import { useTranslation } from '@/i18n';
import appIcon from '@/assets/app-icon.png';
import { ExternalLink } from '@/components/external-link';
import { AUTHOR_URL, PLUMED_NEW_ISSUE_URL } from '@/utils/plumed-links';

// Plumed fork: every page is a direct entry. The Tutorial stays right below PhoneMic.
const getTopLevelItems = (t: (key: string) => string) => [
    {
        name: t('Corrections'),
        url: '/corrections',
        icon: SpellCheck,
        dataTestId: 'corrections-tab',
    },
    {
        name: t('Meeting'),
        url: '/meeting',
        icon: Users,
        dataTestId: 'meeting-tab',
    },
    {
        name: t('PhoneMic'),
        url: '/extensions/remote-mic',
        icon: RadioTower,
        dataTestId: 'remote-mic-tab',
    },
    {
        name: t('Tutorial'),
        url: '/tutorial',
        icon: GraduationCap,
        dataTestId: 'tutorial-tab',
    },
    {
        name: t('Settings'),
        url: '/settings',
        icon: Settings,
        dataTestId: 'settings-tab',
    },
];

export const AppSidebar = () => {
    const { pathname, hash } = useLocation();
    const version = useGetVersion();
    const { isChecking, checkNow } = useCheckAppUpdateNow();
    const { t } = useTranslation();
    const topLevelItems = getTopLevelItems(t);
    const isAboutActive = pathname === '/settings' && hash === 'about';

    return (
        <Sidebar className="bg-background border-border border-r overflow-hidden w-[15.5rem]">
            <SidebarHeader className="flex flex-col items-center justify-center gap-1 bg-background border-b border-border">
                <img src={appIcon} alt="logo" className="w-16 h-16" />
                <ExternalLink
                    href={AUTHOR_URL}
                    withIcon={false}
                    className="text-xs font-medium text-violet-400 no-underline hover:text-violet-300 hover:underline"
                    data-testid="author-link"
                >
                    {t('by Olivier Cornelis')}
                </ExternalLink>
            </SidebarHeader>
            <SidebarContent className="bg-background">
                <SidebarGroup>
                    <SidebarMenu>
                        <SidebarMenuItem>
                            <SidebarMenuButton asChild isActive={pathname === '/'} data-testid="home-tab">
                                <Link to="/">
                                    <Home />
                                    <span>{t('Home')}</span>
                                </Link>
                            </SidebarMenuButton>
                        </SidebarMenuItem>

                        {topLevelItems.map((item) => (
                            <SidebarMenuItem key={item.url}>
                                <SidebarMenuButton
                                    asChild
                                    isActive={pathname === item.url}
                                    data-testid={item.dataTestId}
                                >
                                    <Link to={item.url}>
                                        <item.icon />
                                        <span>{item.name}</span>
                                    </Link>
                                </SidebarMenuButton>
                            </SidebarMenuItem>
                        ))}
                    </SidebarMenu>
                </SidebarGroup>
            </SidebarContent>
            <SidebarFooter className="bg-background ">
                <a
                    href={PLUMED_NEW_ISSUE_URL}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-muted-foreground text-xs hover:text-foreground transition-colors flex items-center gap-2 px-2"
                >
                    <Bug className="w-4 h-4" />
                    <span>{t('Report a bug')}</span>
                </a>
                <Link
                    to="/settings"
                    hash="about"
                    className={`${isAboutActive ? 'text-sky-400' : 'text-muted-foreground hover:text-foreground'} text-xs transition-colors flex items-center gap-2 px-2`}
                    data-testid="about-tab"
                >
                    <Info className="w-4 h-4" />
                    <span>{t('About')}</span>
                </Link>
                <Separator />
                <div className="flex items-center gap-2 justify-center">
                    <UpdateChecker />
                    <button
                        type="button"
                        onClick={() => void checkNow()}
                        disabled={isChecking}
                        title={t('Check for updates')}
                        className="text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer disabled:opacity-50"
                        data-testid="sidebar-version"
                    >
                        {version}
                    </button>
                </div>
            </SidebarFooter>
        </Sidebar>
    );
};
