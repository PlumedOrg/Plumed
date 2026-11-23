import { createRouter, createRoute, createRootRoute, Navigate } from '@tanstack/react-router';
import { Home } from './features/home/home';
import { Layout } from './features/layout/layout';
import { About } from './features/about/about';
import { Corrections } from './features/corrections/corrections';
import { System } from './features/settings/system/system';
import { RemoteMic } from './features/extensions/remote-mic/remote-mic';
import { Tutorial } from './features/tutorial/tutorial';

const rootRoute = createRootRoute({
    component: () => <Layout />,
});

const indexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/',
    component: Home,
});

// Plumed: the shortcuts moved into a tab of the Settings page
const settingsShortcutsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings/shortcuts',
    component: () => <Navigate to="/settings" hash="shortcuts" />,
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

const settingsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings',
    component: System,
});

// Plumed: the old System page path
const settingsSystemRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings/system',
    component: () => <Navigate to="/settings" />,
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

// Plumed: import / export became the last tab of the Settings page
const settingsImportExportRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/settings/import-export',
    component: () => <Navigate to="/settings" hash="import-export" />,
});

const aboutRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/about',
    component: About,
});

const tutorialRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/tutorial',
    component: Tutorial,
});

const routeTree = rootRoute.addChildren([
    indexRoute,
    tutorialRoute,
    settingsRoute,
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
