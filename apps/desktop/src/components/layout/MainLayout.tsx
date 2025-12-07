import { ReactNode } from 'react';
import { Sidebar } from './Sidebar';
import { Bell, Moon, Sun } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useTheme } from '@/hooks/useTheme';
import { LanguageSwitcher } from '@/components/LanguageSwitcher';
import { useTranslation } from 'react-i18next';

interface MainLayoutProps {
  children: ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  const { theme, setTheme } = useTheme();
  const { t } = useTranslation();

  return (
    <div className="flex h-screen overflow-hidden bg-sidebar text-foreground">
      {/* Sidebar sits directly on the base background */}
      <Sidebar />
      
      {/* Main Content Area - Floating Canvas Style */}
      <main className="flex-1 min-w-0 p-2 pl-0 h-screen overflow-hidden relative">
        <div className="h-full w-full bg-background rounded-2xl shadow-sm border border-border/50 overflow-hidden flex flex-col relative">
          {/* Window Drag Region */}
          <div 
            className="absolute top-0 left-0 right-0 h-8 z-40" 
            data-tauri-drag-region 
          />

          {/* Global Actions (Top Right) */}
          <div className="absolute top-3 right-4 z-50 flex items-center gap-2">
            <LanguageSwitcher />
            
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 rounded-full hover:bg-muted"
              onClick={() => setTheme(theme === 'light' ? 'dark' : 'light')}
              title={t('settings.theme')}
            >
              {theme === 'light' ? <Moon className="h-4 w-4" /> : <Sun className="h-4 w-4" />}
              <span className="sr-only">Toggle theme</span>
            </Button>

            <Button variant="ghost" size="icon" className="h-8 w-8 rounded-full hover:bg-muted">
              <Bell className="h-4 w-4" />
              <span className="sr-only">Notifications</span>
            </Button>
          </div>
          
          <div className="flex-1 overflow-auto p-0 pt-8 scrollbar-hide relative z-0">
            {children}
          </div>
        </div>
      </main>
    </div>
  );
}
