import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { useTheme } from '@/hooks/useTheme';
import { useTranslation } from 'react-i18next';
import { Moon, Sun, Monitor } from 'lucide-react';
import { useState, useEffect } from 'react';
import { toast } from 'sonner';

export function SettingsPage() {
  const { theme, setTheme } = useTheme();
  const { t, i18n } = useTranslation();
  const [cacheAgeHours, setCacheAgeHours] = useState<number>(1);
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);

  useEffect(() => {
    const stored = localStorage.getItem('maxCacheAgeHours');
    if (stored) {
      setCacheAgeHours(parseInt(stored, 10));
    }

    const sidebarStored = localStorage.getItem('sidebarCollapsed');
    setSidebarCollapsed(sidebarStored === 'true');
  }, []);

  const handleLanguageChange = (lang: string) => {
    i18n.changeLanguage(lang);
    localStorage.setItem('language', lang);
    toast.success(t('common.success'));
  };

  const handleThemeChange = (newTheme: 'light' | 'dark' | 'system') => {
    setTheme(newTheme);
    toast.success(t('common.success'));
  };

  const handleCacheAgeChange = () => {
    localStorage.setItem('maxCacheAgeHours', cacheAgeHours.toString());
    toast.success(t('settings.cacheAgeSaved'));
  };

  const handleSidebarToggle = (checked: boolean) => {
    setSidebarCollapsed(checked);
    localStorage.setItem('sidebarCollapsed', checked.toString());
    // Dispatch custom event to notify sidebar
    window.dispatchEvent(new Event('sidebarToggle'));
    toast.success(t('common.success'));
  };

  return (
    <div className="space-y-6 w-full">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">{t('settings.title')}</h1>
        <p className="text-muted-foreground mt-2">{t('settings.description')}</p>
      </div>

      {/* Appearance Settings */}
      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle>{t('settings.appearance')}</CardTitle>
          <CardDescription>{t('settings.appearanceDescription')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Language */}
          <div className="space-y-2">
            <Label>{t('settings.language')}</Label>
            <Select value={i18n.language} onValueChange={handleLanguageChange}>
              <SelectTrigger className="rounded-lg">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="zh-CN">{t('settings.chinese')}</SelectItem>
                <SelectItem value="en-US">{t('settings.english')}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Theme */}
          <div className="space-y-2">
            <Label>{t('settings.theme')}</Label>
            <div className="grid grid-cols-3 gap-3">
              <Button
                variant={theme === 'light' ? 'default' : 'outline'}
                onClick={() => handleThemeChange('light')}
                className="rounded-full"
              >
                <Sun className="h-4 w-4 mr-2" />
                {t('settings.light')}
              </Button>
              <Button
                variant={theme === 'dark' ? 'default' : 'outline'}
                onClick={() => handleThemeChange('dark')}
                className="rounded-full"
              >
                <Moon className="h-4 w-4 mr-2" />
                {t('settings.dark')}
              </Button>
              <Button
                variant={theme === 'system' ? 'default' : 'outline'}
                onClick={() => handleThemeChange('system')}
                className="rounded-full"
              >
                <Monitor className="h-4 w-4 mr-2" />
                {t('settings.system')}
              </Button>
            </div>
          </div>

          {/* Sidebar Mode */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="sidebar-mode">{t('settings.sidebarMode')}</Label>
                <p className="text-xs text-muted-foreground">
                  {t('settings.sidebarModeDescription')}
                </p>
              </div>
              <Switch
                id="sidebar-mode"
                checked={sidebarCollapsed}
                onCheckedChange={handleSidebarToggle}
              />
            </div>
            <p className="text-xs text-muted-foreground">
              {sidebarCollapsed 
                ? t('settings.sidebarIconOnly')
                : t('settings.sidebarIconWithText')}
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Data & Performance Settings */}
      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle>{t('settings.dataPerformance')}</CardTitle>
          <CardDescription>{t('settings.dataPerformanceDescription')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Cache Age */}
          <div className="space-y-2">
            <Label htmlFor="cache-age">{t('settings.cacheAge')}</Label>
            <div className="flex items-center gap-3">
              <Input
                id="cache-age"
                type="number"
                min="1"
                max="24"
                value={cacheAgeHours}
                onChange={(e) => setCacheAgeHours(parseInt(e.target.value, 10))}
                className="rounded-lg max-w-[200px]"
              />
              <span className="text-sm text-muted-foreground">{t('settings.hours')}</span>
              <Button onClick={handleCacheAgeChange} className="rounded-full ml-auto">
                {t('common.save')}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              {t('settings.cacheAgeDescription')}
            </p>
          </div>
        </CardContent>
      </Card>

      {/* About */}
      <Card className="rounded-2xl">
        <CardHeader>
          <CardTitle>{t('settings.about')}</CardTitle>
          <CardDescription>{t('settings.aboutDescription')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-2">
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">{t('settings.version')}</span>
            <span className="font-medium">v0.1.0</span>
          </div>
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">{t('settings.copyright')}</span>
            <span className="font-medium">© 2025 NeuraDock</span>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
