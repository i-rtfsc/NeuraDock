import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { QueryClientProvider } from '@tanstack/react-query';
import { queryClient } from './lib/query-client';
import { ThemeProvider } from './hooks/useTheme';
import { MainLayout } from './components/layout/MainLayout';
import { Toaster } from './components/ui/toaster';
import { HomePage } from './pages/HomePage';
import { AccountsPage } from './pages/AccountsPage';
import { AccountOverviewPage } from './pages/AccountOverviewPage';
import { AccountActivityPage } from './pages/AccountActivityPage';
import { TokensPage } from './pages/TokensPage';
import { RelayStationsPage } from './pages/RelayStationsPage';
import { PreferencesPage } from './pages/PreferencesPage';
import { TooltipProvider } from '@/components/ui/tooltip';

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <TooltipProvider delayDuration={0}>
          <BrowserRouter>
            <MainLayout>
              <Routes>
                <Route path="/" element={<HomePage />} />
                <Route path="/accounts" element={<AccountsPage />} />
                <Route path="/accounts/:accountId" element={<AccountOverviewPage />} />
                <Route path="/account/:accountId/records" element={<AccountActivityPage />} />
                <Route path="/tokens" element={<TokensPage />} />
                <Route path="/providers" element={<RelayStationsPage />} />
                <Route path="/settings" element={<PreferencesPage />} />
              </Routes>
            </MainLayout>
            <Toaster />
          </BrowserRouter>
        </TooltipProvider>
      </ThemeProvider>
    </QueryClientProvider>
  );
}

export default App;
