import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { QueryClientProvider } from '@tanstack/react-query';
import { queryClient } from './lib/query-client';
import { ThemeProvider } from './hooks/useTheme';
import { MainLayout } from './components/layout/MainLayout';
import { Toaster } from './components/ui/toaster';
import { DashboardPage } from './pages/DashboardPage';
import { AccountsTablePage } from './pages/AccountsTablePage';
import { AccountDetailPage } from './pages/AccountDetailPage';
import { AccountRecordsPage } from './pages/AccountRecordsPage';
import { TokenManagerPage } from './pages/TokenManagerPage';
import { SettingsPage } from './pages/SettingsPage';
import { TooltipProvider } from '@/components/ui/tooltip';

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <TooltipProvider delayDuration={0}>
          <BrowserRouter>
            <MainLayout>
              <Routes>
                <Route path="/" element={<DashboardPage />} />
                <Route path="/accounts" element={<AccountsTablePage />} />
                <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
                <Route path="/account/:accountId/records" element={<AccountRecordsPage />} />
                <Route path="/tokens" element={<TokenManagerPage />} />
                <Route path="/settings" element={<SettingsPage />} />
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
