import { Suspense, lazy } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { QueryClientProvider } from '@tanstack/react-query';
import { queryClient } from './lib/query-client';
import { ThemeProvider } from './hooks/useTheme';
import { MainLayout } from './components/layout/MainLayout';
import { Toaster } from './components/ui/toaster';
import { TooltipProvider } from '@/components/ui/tooltip';
import { LoadingState } from './components/ui/loading';

const HomePage = lazy(() => import('./pages/HomePage').then((m) => ({ default: m.HomePage })));
const AccountsPage = lazy(() =>
  import('./pages/AccountsPage').then((m) => ({ default: m.AccountsPage }))
);
const AccountOverviewPage = lazy(() =>
  import('./pages/AccountOverviewPage').then((m) => ({ default: m.AccountOverviewPage }))
);
const AccountActivityPage = lazy(() =>
  import('./pages/AccountActivityPage').then((m) => ({ default: m.AccountActivityPage }))
);
const TokensPage = lazy(() => import('./pages/TokensPage').then((m) => ({ default: m.TokensPage })));
const ProvidersPage = lazy(() =>
  import('./pages/ProvidersPage').then((m) => ({ default: m.ProvidersPage }))
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

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <TooltipProvider delayDuration={0}>
          <BrowserRouter>
            <MainLayout>
              <Suspense fallback={<LoadingState className="h-full" />}>
                <Routes>
                  <Route path="/" element={<HomePage />} />
                  <Route path="/accounts" element={<AccountsPage />} />
                  <Route path="/accounts/:accountId" element={<AccountOverviewPage />} />
                  <Route path="/account/:accountId/records" element={<AccountActivityPage />} />
                  <Route path="/tokens" element={<TokensPage />} />
                  <Route path="/providers" element={<ProvidersPage />} />
                  <Route path="/ai-chat" element={<AiChatPage />} />
                  <Route path="/ai-chat/settings" element={<AiChatSettingsPage />} />
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
