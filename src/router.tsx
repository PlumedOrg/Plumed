import { createRouter, createRoute, createRootRoute, Navigate } from '@tanstack/react-router';
import { Home } from './features/home/home';
import { Layout } from './features/layout/layout';
import { About } from './features/about/about';
import { Shortcuts } from './features/settings/shortcuts/shortcuts';
import { Corrections } from './features/corrections/corrections';
import { System } from './features/settings/system/system';
import { ImportExport } from './features/settings/import-export/import-export';
import { RemoteMic } from './features/extensions/remote-mic/remote-mic';

const rootRoute = createRootRoute({
    component: () => <Layout />,
});

const indexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/',
    component: Home,
});

const settingsShortcutsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings/shortcuts',
    component: Shortcuts,
});

const correctionsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/corrections',
    component: Corrections,
});

// Plumed: the dictionary and the formatting rules moved into the Corrections page
const personalizeCustomDictionaryRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/personalize/custom-dictionary',
    component: () => <Navigate to="/corrections" hash="dictionary" />,
});

const personalizeFormattingRulesRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/personalize/formatting-rules',
    component: () => <Navigate to="/corrections" hash="formatting" />,
});

const settingsSystemRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings/system',
    component: System,
});

const settingsIndexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings',
    component: () => <Navigate to="/settings/shortcuts" />,
});

const personalizeIndexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/personalize',
    component: () => <Navigate to="/corrections" hash="dictionary" />,
});

const extensionsRemoteMicRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/extensions/remote-mic',
    component: RemoteMic,
});

const extensionsIndexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/extensions',
    component: () => <Navigate to="/extensions/remote-mic" />,
});

const settingsImportExportRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings/import-export',
    component: ImportExport,
});

const aboutRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/about',
    component: About,
});

const routeTree = rootRoute.addChildren([
    indexRoute,
    settingsIndexRoute,
    settingsShortcutsRoute,
    settingsSystemRoute,
    settingsImportExportRoute,
    correctionsRoute,
    personalizeIndexRoute,
    personalizeCustomDictionaryRoute,
    personalizeFormattingRulesRoute,
    extensionsIndexRoute,
    extensionsRemoteMicRoute,
    aboutRoute,
]);

export const router = createRouter({ routeTree });
