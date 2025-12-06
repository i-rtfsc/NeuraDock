import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { useTheme } from '@/hooks/useTheme';
import { useTranslation } from 'react-i18next';
import { Moon, Sun, Monitor, Palette, Zap, Code, Bell, Info, ChevronRight, AlertTriangle, Scale } from 'lucide-react';
import { useState, useEffect } from 'react';
import { toast } from 'sonner';
import { invoke } from '@tauri-apps/api/core';
import { NotificationChannelList } from '@/components/notification/NotificationChannelList';
import { useNotificationChannels } from '@/hooks/useNotificationChannels';
import { cn } from '@/lib/utils';

type SettingSection = 'appearance' | 'performance' | 'notifications' | 'developer' | 'about';

interface NavigationItem {
  id: SettingSection;
  icon: React.ElementType;
  labelKey: string;
  descKey: string;
}

const navigationItems: NavigationItem[] = [
  { id: 'appearance', icon: Palette, labelKey: 'settings.appearance', descKey: 'settings.appearanceDescription' },
  { id: 'performance', icon: Zap, labelKey: 'settings.dataPerformance', descKey: 'settings.dataPerformanceDescription' },
  { id: 'notifications', icon: Bell, labelKey: 'settings.notification', descKey: 'settings.notificationDescription' },
  { id: 'developer', icon: Code, labelKey: 'settings.developer', descKey: 'settings.developerDescription' },
  { id: 'about', icon: Info, labelKey: 'settings.about', descKey: 'settings.aboutDescription' },
];

export function SettingsPage() {
  const { theme, setTheme } = useTheme();
  const { t, i18n } = useTranslation();
  const [activeSection, setActiveSection] = useState<SettingSection>('appearance');
  const [cacheAgeHours, setCacheAgeHours] = useState<number>(1);
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);
  const [logLevel, setLogLevel] = useState<string>('info');

  // Load notification channels
  const { data: notificationChannels = [], refetch: refetchChannels } = useNotificationChannels();

  useEffect(() => {
    const stored = localStorage.getItem('maxCacheAgeHours');
    if (stored) {
      setCacheAgeHours(parseInt(stored, 10));
    }

    const sidebarStored = localStorage.getItem('sidebarCollapsed');
    setSidebarCollapsed(sidebarStored === 'true');

    // Load log level
    invoke<string>('get_log_level').then(level => {
      setLogLevel(level);
    }).catch(err => {
      console.error('Failed to get log level:', err);
    });
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
    window.dispatchEvent(new Event('sidebarToggle'));
    toast.success(t('common.success'));
  };

  const handleLogLevelChange = async (level: string) => {
    try {
      await invoke('set_log_level', { level });
      setLogLevel(level);
      toast.success(t('settings.logLevelUpdated'), {
        description: t('settings.restartRequired'),
      });
    } catch (err) {
      toast.error(t('common.error'), {
        description: String(err),
      });
    }
  };

  const renderContent = () => {
    switch (activeSection) {
      case 'appearance':
        return (
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
        );

      case 'performance':
        return (
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle>{t('settings.dataPerformance')}</CardTitle>
              <CardDescription>{t('settings.dataPerformanceDescription')}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
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
        );

      case 'notifications':
        return (
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle>{t('settings.notification')}</CardTitle>
              <CardDescription>{t('settings.notificationDescription')}</CardDescription>
            </CardHeader>
            <CardContent>
              <NotificationChannelList
                channels={notificationChannels}
                onUpdate={refetchChannels}
              />
            </CardContent>
          </Card>
        );

      case 'developer':
        return (
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Code className="h-5 w-5" />
                {t('settings.developer')}
              </CardTitle>
              <CardDescription>{t('settings.developerDescription')}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {/* Log Level */}
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label className="text-base font-semibold">{t('settings.logLevel')}</Label>
                  <Badge variant="outline" className="rounded-full">
                    {logLevel.toUpperCase()}
                  </Badge>
                </div>
                
                <Select value={logLevel} onValueChange={handleLogLevelChange}>
                  <SelectTrigger className="rounded-xl h-11">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent className="rounded-xl">
                    <SelectItem value="error" className="rounded-lg">
                      <div className="flex items-center gap-2">
                        <span className="text-red-500">●</span>
                        {t('settings.logLevelError')}
                      </div>
                    </SelectItem>
                    <SelectItem value="warn" className="rounded-lg">
                      <div className="flex items-center gap-2">
                        <span className="text-amber-500">●</span>
                        {t('settings.logLevelWarn')}
                      </div>
                    </SelectItem>
                    <SelectItem value="info" className="rounded-lg">
                      <div className="flex items-center gap-2">
                        <span className="text-blue-500">●</span>
                        {t('settings.logLevelInfo')}
                      </div>
                    </SelectItem>
                    <SelectItem value="debug" className="rounded-lg">
                      <div className="flex items-center gap-2">
                        <span className="text-purple-500">●</span>
                        {t('settings.logLevelDebug')}
                      </div>
                    </SelectItem>
                    <SelectItem value="trace" className="rounded-lg">
                      <div className="flex items-center gap-2">
                        <span className="text-gray-500">●</span>
                        {t('settings.logLevelTrace')}
                      </div>
                    </SelectItem>
                  </SelectContent>
                </Select>

                <Alert className="rounded-xl border-2">
                  <AlertTriangle className="h-4 w-4" />
                  <AlertDescription className="text-sm">
                    {t('settings.logLevelDescription')}
                  </AlertDescription>
                </Alert>

                <Alert variant="warning" className="rounded-xl border-2">
                  <Info className="h-4 w-4" />
                  <AlertDescription className="text-sm font-medium">
                    {t('settings.restartRequired')}
                  </AlertDescription>
                </Alert>
              </div>
            </CardContent>
          </Card>
        );

      case 'about':
        return (
          <div className="space-y-6">
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

            {/* Disclaimer */}
            <Alert variant="warning" className="border-2 rounded-2xl">
              <AlertTriangle className="h-5 w-5" />
              <AlertDescription className="space-y-4 pt-2">
                <div className="font-bold text-base">{t('disclaimer.title')}</div>
                <div className="space-y-2">
                  <p className="text-sm font-semibold">
                    {t('disclaimer.liability.title')}
                  </p>
                  <p className="text-sm">{t('disclaimer.liability.description')}</p>
                  <p className="text-sm font-semibold">
                    ⚠️ {t('disclaimer.liability.warning')}
                  </p>
                </div>
                <div className="space-y-2 text-sm">
                  <div className="flex items-center gap-2 font-semibold">
                    <Scale className="h-4 w-4" />
                    {t('disclaimer.license.title')}
                  </div>
                  <p>{t('disclaimer.license.description')}</p>
                  <p className="font-semibold">{t('disclaimer.license.commercial')}</p>
                  <p className="text-muted-foreground italic">
                    {t('disclaimer.license.footer')}
                  </p>
                </div>
              </AlertDescription>
            </Alert>
          </div>
        );

      default:
        return null;
    }
  };

  return (
    <div className="space-y-6 w-full">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold tracking-tight">{t('settings.title')}</h1>
        <p className="text-muted-foreground mt-2">{t('settings.description')}</p>
      </div>

      {/* Main Layout: Sidebar + Content */}
      <div className="flex gap-6">
        {/* Left Sidebar Navigation */}
        <nav className="w-64 shrink-0">
          <Card className="rounded-2xl p-2">
            <div className="space-y-1">
              {navigationItems.map((item) => {
                const Icon = item.icon;
                const isActive = activeSection === item.id;

                return (
                  <button
                    key={item.id}
                    onClick={() => setActiveSection(item.id)}
                    className={cn(
                      "w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-left",
                      isActive
                        ? "bg-primary text-primary-foreground shadow-sm"
                        : "text-muted-foreground hover:bg-accent/50 hover:text-foreground"
                    )}
                  >
                    <Icon className="h-5 w-5 shrink-0" />
                    <div className="flex-1 min-w-0">
                      <div className={cn(
                        "text-sm font-medium truncate",
                        isActive && "font-semibold"
                      )}>
                        {t(item.labelKey)}
                      </div>
                      <div className={cn(
                        "text-xs truncate mt-0.5",
                        isActive
                          ? "text-primary-foreground/80"
                          : "text-muted-foreground/70"
                      )}>
                        {t(item.descKey)}
                      </div>
                    </div>
                    {isActive && (
                      <ChevronRight className="h-4 w-4 shrink-0" />
                    )}
                  </button>
                );
              })}
            </div>
          </Card>
        </nav>

        {/* Right Content Area */}
        <div className="flex-1 min-w-0">
          {renderContent()}
        </div>
      </div>
    </div>
  );
}
