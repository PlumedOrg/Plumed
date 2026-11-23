import {
    Home,
    Settings,
    Info,
    ChevronRight,
    Power,
    Bug,
    Newspaper,
    ArrowDownUp,
    RadioTower,
    SpellCheck,
    GraduationCap,
} from 'lucide-react';
import { Link, useLocation } from '@tanstack/react-router';
import { useState } from 'react';
import {
    Sidebar,
    SidebarHeader,
    SidebarContent,
    SidebarFooter,
    SidebarGroup,
    SidebarMenu,
    SidebarMenuItem,
    SidebarMenuButton,
    SidebarMenuSub,
    SidebarMenuSubItem,
    SidebarMenuSubButton,
} from '../../../components/sidebar';
import { useGetVersion } from '../hooks/use-get-version';
import { UpdateChecker } from '../../update-checker/update-checker';
import { Separator } from '@/components/separator';
import { useTranslation } from '@/i18n';
import { useDictionaryRevampSeen } from '@/features/personalize/custom-dictionary/hooks/use-dictionary-revamp-seen';
import appIcon from '@/assets/app-icon.png';
import { isPathInItems } from './app-sidebar.helpers';
import { PLUMED_LATEST_RELEASE_URL, PLUMED_NEW_ISSUE_URL, PLUMED_RELEASES_URL } from '@/utils/plumed-links';

const getTopLevelItems = (t: (key: string) => string) => [
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
];

const getSettingsSubItems = (t: (key: string) => string) => [
    {
        name: t('Corrections'),
        url: '/corrections',
        icon: SpellCheck,
        dataTestId: 'corrections-tab',
    },
    {
        name: t('System'),
        url: '/settings/system',
        icon: Power,
        dataTestId: 'system-tab',
    },
    {
        name: t('Import / Export'),
        url: '/settings/import-export',
        icon: ArrowDownUp,
        dataTestId: 'import-export-tab',
    },
];

export const AppSidebar = () => {
    const { pathname } = useLocation();
    const [settingsOpen, setSettingsOpen] = useState(true);
    const version = useGetVersion();
    const { t } = useTranslation();
    const { seen: revampSeen } = useDictionaryRevampSeen();
    const topLevelItems = getTopLevelItems(t);
    const settingsSubItems = getSettingsSubItems(t);
    const showRevampDot = !revampSeen;
    const isSettingsActive = isPathInItems(pathname, settingsSubItems);
    const isAboutActive = pathname === '/about';

    return (
        <Sidebar className="bg-background border-border border-r overflow-hidden w-[15.5rem]">
            <SidebarHeader className="flex items-center justify-center bg-background border-b border-border">
                <img src={appIcon} alt="logo" className="w-16 h-16" />
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

                        <SidebarMenuItem>
                            <SidebarMenuButton
                                onClick={() => setSettingsOpen(!settingsOpen)}
                                data-testid="settings-tab"
                            >
                                <Settings className={isSettingsActive ? 'text-sky-400' : undefined} />
                                <span>{t('Settings')}</span>
                                <ChevronRight
                                    className={`ml-auto transition-transform ${settingsOpen ? 'rotate-90' : ''}`}
                                />
                            </SidebarMenuButton>
                            {settingsOpen && (
                                <SidebarMenuSub>
                                    {settingsSubItems.map((item) => (
                                        <SidebarMenuSubItem key={item.url} data-testid={item.dataTestId}>
                                            <SidebarMenuSubButton asChild isActive={pathname === item.url}>
                                                <Link to={item.url}>
                                                    <item.icon />
                                                    <span>{item.name}</span>
                                                    {item.url === '/corrections' && showRevampDot && (
                                                        <span
                                                            className="ml-auto h-2 w-2 rounded-full bg-sky-500"
                                                            title={t('Improved')}
                                                            aria-label={t('Improved')}
                                                        />
                                                    )}
                                                </Link>
                                            </SidebarMenuSubButton>
                                        </SidebarMenuSubItem>
                                    ))}
                                </SidebarMenuSub>
                            )}
                        </SidebarMenuItem>
                    </SidebarMenu>
                </SidebarGroup>
            </SidebarContent>
            <SidebarFooter className="bg-background ">
                <a
                    href={version.length > 0 ? `${PLUMED_RELEASES_URL}/tag/${version}` : PLUMED_LATEST_RELEASE_URL}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-muted-foreground text-xs hover:text-foreground transition-colors flex items-center gap-2 px-2"
                >
                    <Newspaper className="w-4 h-4" />
                    <span>{t('Release notes')}</span>
                </a>
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
                    to="/about"
                    className={`${isAboutActive ? 'text-sky-400' : 'text-muted-foreground hover:text-foreground'} text-xs transition-colors flex items-center gap-2 px-2`}
                    data-testid="about-tab"
                >
                    <Info className="w-4 h-4" />
                    <span>{t('About')}</span>
                </Link>
                <Separator />
                <div className="flex items-center gap-2 justify-center">
                    <UpdateChecker />
                    <p className="text-xs text-muted-foreground">{version}</p>
                </div>
            </SidebarFooter>
        </Sidebar>
    );
};
