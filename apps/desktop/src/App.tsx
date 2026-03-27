import { Suspense, lazy } from 'react';
import { BrowserRouter, Navigate, Route, Routes, useSearchParams } from 'react-router-dom';
import { QueryClientProvider } from '@tanstack/react-query';
import { queryClient } from './lib/query-client';
import { ThemeProvider } from './hooks/useTheme';
import { MainLayout } from './components/layout/MainLayout';
import { Toaster } from './components/ui/toaster';
import { TooltipProvider } from '@/components/ui/tooltip';
import { LoadingState } from './components/ui/loading';

const AccountOverviewPage = lazy(() =>
  import('./pages/AccountOverviewPage').then((m) => ({ default: m.AccountOverviewPage }))
);
const AccountActivityPage = lazy(() =>
  import('./pages/AccountActivityPage').then((m) => ({ default: m.AccountActivityPage }))
);
const TokensPage = lazy(() =>
  import('./pages/TokensPage').then((m) => ({ default: m.TokensPage }))
);
const TransitHubPage = lazy(() =>
  import('./pages/TransitHubPage').then((m) => ({ default: m.TransitHubPage }))
);
const DevToolboxPage = lazy(() =>
  import('./pages/DevToolboxPage').then((m) => ({ default: m.DevToolboxPage }))
);
const PreferencesPage = lazy(() =>
  import('./pages/PreferencesPage').then((m) => ({ default: m.PreferencesPage }))
);
const AiChatSettingsPage = lazy(() =>
  import('./pages/AiChatSettingsPage').then((m) => ({ default: m.AiChatSettingsPage }))
);
const AiChatPage = lazy(() =>
  import('./pages/AiChatPage').then((m) => ({ default: m.AiChatPage }))
);
const CalendarPage = lazy(() =>
  import('./pages/CalendarPage').then((m) => ({ default: m.CalendarPage }))
);
const CodexPage = lazy(() =>
  import('./pages/CodexPage').then((m) => ({ default: m.CodexPage }))
);
import { normalizeAiToolboxSection } from './lib/aiToolbox';
import { buildTransitHubPath, normalizeTransitHubTab } from './lib/transitHub';

function LegacyAiToolsRedirect() {
  const [searchParams] = useSearchParams();
  const section = normalizeAiToolboxSection(searchParams.get('section'));

  switch (section) {
    case 'tokens':
      return <Navigate to="/tokens" replace />;
    case 'chat':
      return <Navigate to="/ai-chat" replace />;
    case 'codex':
      return <Navigate to="/codex" replace />;
    case 'transit':
    default:
      return <Navigate to={buildTransitHubPath(normalizeTransitHubTab(searchParams.get('tab')))} replace />;
  }
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <TooltipProvider delayDuration={0}>
          <BrowserRouter>
            <MainLayout>
              <Suspense fallback={<LoadingState className="h-full" />}>
                <Routes>
                  <Route path="/" element={<Navigate to={buildTransitHubPath()} replace />} />
                  <Route path="/accounts" element={<Navigate to={buildTransitHubPath('accounts')} replace />} />
                  <Route path="/accounts/:accountId" element={<AccountOverviewPage />} />
                  <Route path="/account/:accountId/records" element={<AccountActivityPage />} />
                  <Route path="/providers" element={<TransitHubPage />} />
                  <Route path="/ai-tools" element={<LegacyAiToolsRedirect />} />
                  <Route path="/tokens" element={<TokensPage />} />
                  <Route path="/dev-tools" element={<DevToolboxPage />} />
                  <Route path="/daily-tools" element={<Navigate to="/calendar" replace />} />
                  <Route path="/calendar" element={<CalendarPage />} />
                  <Route path="/ai-chat" element={<AiChatPage />} />
                  <Route path="/ai-chat/settings" element={<AiChatSettingsPage />} />
                  <Route path="/codex" element={<CodexPage />} />
                  <Route path="/settings" element={<PreferencesPage />} />
                </Routes>
              </Suspense>
            </MainLayout>
            <Toaster />
          </BrowserRouter>
        </TooltipProvider>
      </ThemeProvider>
    </QueryClientProvider>
  );
}

export default App;
