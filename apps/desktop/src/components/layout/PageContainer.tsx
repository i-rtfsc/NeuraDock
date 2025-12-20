import { cn } from '@/lib/utils';
import { ReactNode } from 'react';
import { Moon, Sun } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useTheme } from '@/hooks/useTheme';
import { LanguageSwitcher } from '@/components/LanguageSwitcher';
import { useTranslation } from 'react-i18next';

interface PageContainerProps {
  children: ReactNode;
  className?: string;
  title?: ReactNode;
  actions?: ReactNode;
  headerClassName?: string;
}

export function PageContainer({ children, className, title, actions, headerClassName }: PageContainerProps) {
  const { theme, setTheme } = useTheme();
  const { t } = useTranslation();

  return (
    <div className="flex flex-col h-full">
      {/* Header - 使用design tokens统一高度和padding */}
      <div className={cn(
        "flex items-center justify-between shrink-0 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 z-10",
        "px-[var(--layout-page-header-padding-x)] py-[var(--layout-page-header-padding-y)]",
        "gap-[var(--spacing-element-gap)]",
        "h-[var(--layout-page-header-height)]",
        headerClassName,
      )} data-tauri-drag-region>
        <div
          data-tauri-drag-region
          className="flex items-center gap-[var(--spacing-element-gap)] min-w-0 shrink-0"
        >
          {typeof title === 'string' ? (
            <h1 className="text-2xl font-bold tracking-tight truncate">{title}</h1>
          ) : (
            title
          )}
        </div>

        <div
          data-tauri-drag-region
          className="flex items-center flex-1 justify-end min-w-0 gap-[var(--spacing-element-gap)]"
        >
          {/* Page Actions (Search, Tabs, Buttons) */}
          {actions}

          {/* Separator */}
          <div className="w-px h-6 bg-border shrink-0" />

          {/* Global Actions */}
          <div className="flex items-center gap-[var(--spacing-element-gap-sm)] shrink-0">
            <LanguageSwitcher />

            <Button
              variant="ghost"
              size="icon-sm"
              className="rounded-full hover:bg-muted"
              onClick={() => setTheme(theme === 'light' ? 'dark' : 'light')}
              title={t('settings.theme')}
            >
              {theme === 'light' ? <Moon className="h-4 w-4" /> : <Sun className="h-4 w-4" />}
              <span className="sr-only">Toggle theme</span>
            </Button>
          </div>
        </div>
      </div>

      {/* Content - 使用design tokens统一padding */}
      <div className={cn("flex-1 overflow-auto pt-[var(--layout-page-content-padding-top)] px-[var(--layout-page-content-padding)] pb-[var(--layout-page-content-padding)]", className)}>
        {children}
      </div>
    </div>
  );
}
