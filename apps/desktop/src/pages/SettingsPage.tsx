import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { useTheme } from '@/hooks/useTheme';
import { useTranslation } from 'react-i18next';
import {
  Moon, Sun, Monitor, Palette, Zap, Code, Bell, Info,
  AlertTriangle, Scale,
  Database, Trash2,
  Terminal
} from 'lucide-react';
import { useState, useEffect } from 'react';
import { toast } from 'sonner';
import { invoke } from '@tauri-apps/api/core';
import { NotificationChannelList } from '@/components/notification/NotificationChannelList';
import { useNotificationChannels } from '@/hooks/useNotificationChannels';
import { cn } from '@/lib/utils';
import { PageContainer } from '@/components/layout/PageContainer';
import { SidebarPageLayout } from '@/components/layout/SidebarPageLayout';
import { ScrollArea } from '@/components/ui/scroll-area';

// --- Types & Constants ---
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

// --- Sub-Components ---

const AppearanceSettings = () => {
  const { theme, setTheme } = useTheme();
  const { t, i18n } = useTranslation();
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => {
    return localStorage.getItem('sidebarCollapsed') === 'true';
  });

  const handleSidebarToggle = (checked: boolean) => {
    setSidebarCollapsed(checked);
    localStorage.setItem('sidebarCollapsed', checked.toString());
    window.dispatchEvent(new Event('sidebarToggle'));
    toast.success(t('common.success'));
  };

  const handleLanguageChange = (lang: string) => {
    i18n.changeLanguage(lang);
    localStorage.setItem('language', lang);
    toast.success(t('common.success'));
  };

  return (
    <div className="space-y-10">
      {/* Header */}
      <div className="space-y-1">
        <h2 className="text-2xl font-bold tracking-tight">{t('settings.appearance')}</h2>
        <p className="text-muted-foreground text-sm">{t('settings.appearanceDescription')}</p>
      </div>
      
      {/* Theme Selection - Stable Icon Cards */}
      <section className="space-y-4">
        <Label className="text-base font-semibold">{t('settings.theme')}</Label>
        <div className="grid grid-cols-3 gap-4">
          {[
            { value: 'light', icon: Sun, label: t('settings.light') },
            { value: 'dark', icon: Moon, label: t('settings.dark') },
            { value: 'system', icon: Monitor, label: t('settings.system') },
          ].map((item) => (
            <div
              key={item.value}
              onClick={() => setTheme(item.value as 'light' | 'dark' | 'system')}
              className={cn(
                "cursor-pointer flex flex-col items-center gap-3 rounded-xl border-2 p-4 transition-all hover:bg-accent hover:text-accent-foreground",
                theme === item.value 
                  ? "border-primary bg-primary/5" 
                  : "border-muted bg-card"
              )}
            >
              <div className={cn(
                "p-3 rounded-full ring-1 ring-border",
                theme === item.value ? "bg-primary text-primary-foreground ring-primary border-transparent" : "bg-background"
              )}>
                <item.icon className="h-6 w-6" />
              </div>
              <span className="font-medium text-sm">{item.label}</span>
            </div>
          ))}
        </div>
      </section>

      <div className="h-px bg-border/50" />

      {/* General Settings List */}
      <section className="space-y-6">
         {/* Language */}
         <div className="flex items-center justify-between">
            <div className="space-y-1">
               <Label className="text-base font-medium">{t('settings.language')}</Label>
               <p className="text-sm text-muted-foreground">
                 {t('settings.appearanceDescription').split(' ')[0]} language preference.
               </p>
            </div>
            <Select value={i18n.language} onValueChange={handleLanguageChange}>
              <SelectTrigger className="w-[180px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="zh-CN">
                  🇨🇳 简体中文
                </SelectItem>
                <SelectItem value="en-US">
                  🇺🇸 English
                </SelectItem>
              </SelectContent>
            </Select>
         </div>

         {/* Sidebar Mode */}
         <div className="flex items-center justify-between">
            <div className="space-y-1">
               <Label className="text-base font-medium">{t('settings.sidebarMode')}</Label>
               <p className="text-sm text-muted-foreground max-w-[300px]">
                 {sidebarCollapsed ? t('settings.sidebarCollapsedDesc') : t('settings.sidebarExpandedDesc')}
               </p>
            </div>
            <div className="flex items-center gap-3">
               <span className={cn("text-sm", !sidebarCollapsed && "font-medium")}>{t('settings.sidebarIconWithText')}</span>
               <Switch
                 checked={sidebarCollapsed}
                 onCheckedChange={handleSidebarToggle}
               />
               <span className={cn("text-sm", sidebarCollapsed && "font-medium")}>{t('settings.sidebarIconOnly')}</span>
            </div>
         </div>
      </section>
    </div>
  );
};

const PerformanceSettings = () => {
  const { t } = useTranslation();
  const [cacheAgeHours, setCacheAgeHours] = useState<number>(1);

  useEffect(() => {
    const stored = localStorage.getItem('maxCacheAgeHours');
    if (stored) {
      setCacheAgeHours(parseInt(stored, 10));
    }
  }, []);

  const handleSave = () => {
    localStorage.setItem('maxCacheAgeHours', cacheAgeHours.toString());
    toast.success(t('settings.cacheAgeSaved'));
  };

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div>
        <h2 className="text-2xl font-bold tracking-tight mb-1">{t('settings.dataPerformance')}</h2>
        <p className="text-muted-foreground mb-6">{t('settings.dataPerformanceDescription')}</p>

        <div className="grid gap-6">
          <Card className="border-none shadow-md bg-gradient-to-br from-background to-muted/20 overflow-hidden">
            <div className="absolute top-0 right-0 p-3 opacity-10">
               <Database className="w-32 h-32" />
            </div>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Database className="h-5 w-5 text-primary" />
                {t('settings.cacheControl')}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-6 relative">
              <div className="space-y-4">
                 <div className="flex justify-between items-end">
                    <Label className="text-base">{t('settings.cacheAge')}</Label>
                    <div className="text-2xl font-bold text-primary tabular-nums">
                      {cacheAgeHours} <span className="text-sm font-normal text-muted-foreground">{t('settings.hours')}</span>
                    </div>
                 </div>
                 
                 <div className="pt-2">
                   <input
                     type="range"
                     min="1"
                     max="24"
                     step="1"
                     value={cacheAgeHours}
                     onChange={(e) => setCacheAgeHours(parseInt(e.target.value))}
                     className="w-full h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
                   />
                   <div className="flex justify-between text-xs text-muted-foreground mt-2">
                     <span>1 {t('settings.hour')}</span>
                     <span>12 {t('settings.hours')}</span>
                     <span>24 {t('settings.hours')}</span>
                   </div>
                 </div>

                 <div className="flex items-center justify-between pt-2">
                    <p className="text-sm text-muted-foreground max-w-[70%]">
                      {t('settings.cacheAgeDescription')}
                    </p>
                    <Button onClick={handleSave} className="rounded-full shadow-lg hover:shadow-xl transition-all">
                      {t('common.save')}
                    </Button>
                 </div>
              </div>
            </CardContent>
          </Card>

          {/* Placeholder for future Storage Stats */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <Card className="bg-muted/30 border-none shadow-sm">
               <CardContent className="p-6 flex flex-col items-center justify-center text-center space-y-2 opacity-50">
                  <Database className="h-8 w-8 mb-2" />
                  <div className="font-semibold">Local Database</div>
                  <div className="text-xs">12.5 MB used</div>
               </CardContent>
            </Card>
             <Card className="bg-muted/30 border-none shadow-sm">
               <CardContent className="p-6 flex flex-col items-center justify-center text-center space-y-2 opacity-50">
                  <Trash2 className="h-8 w-8 mb-2" />
                  <div className="font-semibold">Temp Files</div>
                  <div className="text-xs">Empty</div>
               </CardContent>
            </Card>
          </div>
        </div>
      </div>
    </div>
  );
};

const NotificationSettings = () => {
  const { t } = useTranslation();
  const { data: notificationChannels = [], refetch: refetchChannels } = useNotificationChannels();

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500 h-full flex flex-col">
      <div className="flex-none">
        <h2 className="text-2xl font-bold tracking-tight mb-1">{t('settings.notification')}</h2>
        <p className="text-muted-foreground mb-6">{t('settings.notificationDescription')}</p>
      </div>

      <div className="flex-1 min-h-0">
        <NotificationChannelList
          channels={notificationChannels}
          onUpdate={refetchChannels}
        />
      </div>
    </div>
  );
};

const DeveloperSettings = () => {
  const { t } = useTranslation();
  const [logLevel, setLogLevel] = useState<string>('info');
  const [isOpeningLogs, setIsOpeningLogs] = useState(false);

  useEffect(() => {
    invoke<string>('get_log_level').then(setLogLevel).catch(console.error);
  }, []);

  const handleLogLevelChange = async (level: string) => {
    try {
      await invoke('set_log_level', { level });
      setLogLevel(level);
      toast.success(t('settings.logLevelUpdated'));
    } catch (err) {
      toast.error(t('common.error'));
    }
  };

  const handleOpenLogs = async () => {
    try {
      setIsOpeningLogs(true);
      const logPath = await invoke<string>('open_log_dir');
      toast.success(t('settings.logFolderOpened', { path: logPath }));
    } catch (error) {
      toast.error(t('settings.failedToOpenLogs') + ': ' + String(error));
    } finally {
      setIsOpeningLogs(false);
    }
  };

  const levels = ['error', 'warn', 'info', 'debug', 'trace'];
  const levelColors: Record<string, string> = {
    error: 'text-red-500 bg-red-500/10 border-red-500/20 ring-red-500/20',
    warn: 'text-amber-500 bg-amber-500/10 border-amber-500/20 ring-amber-500/20',
    info: 'text-blue-500 bg-blue-500/10 border-blue-500/20 ring-blue-500/20',
    debug: 'text-purple-500 bg-purple-500/10 border-purple-500/20 ring-purple-500/20',
    trace: 'text-gray-500 bg-gray-500/10 border-gray-500/20 ring-gray-500/20',
  };

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div>
        <h2 className="text-2xl font-bold tracking-tight mb-1 flex items-center gap-2">
          <Terminal className="h-6 w-6 text-primary" />
          {t('settings.developer')}
        </h2>
        <p className="text-muted-foreground mb-6">{t('settings.developerDescription')}</p>

        <div className="grid gap-6">
          {/* Log Level Console */}
          <Card className="border-border shadow-sm overflow-hidden bg-card">
             <CardHeader className="bg-muted/30 py-3 px-4 border-b border-border flex flex-row items-center justify-between space-y-0">
                   <span className="text-xs font-mono uppercase tracking-widest text-muted-foreground truncate mr-2">Log Level Configuration</span>
                   <div className="flex gap-1.5 opacity-60 shrink-0">
                      <div className="w-3 h-3 rounded-full bg-red-400/80" />
                      <div className="w-3 h-3 rounded-full bg-amber-400/80" />
                      <div className="w-3 h-3 rounded-full bg-green-400/80" />
                   </div>
             </CardHeader>
             <CardContent className="p-6 space-y-6">
                <div className="flex flex-wrap gap-2">
                   {levels.map(level => (
                     <button
                       key={level}
                       onClick={() => handleLogLevelChange(level)}
                       className={cn(
                         "px-4 py-2 rounded-md font-mono text-sm transition-all border",
                         logLevel === level
                           ? levelColors[level] + " ring-1 ring-offset-1 ring-offset-background font-bold shadow-sm"
                           : "bg-muted/40 border-transparent text-muted-foreground hover:bg-muted hover:text-foreground"
                       )}
                     >
                       {level.toUpperCase()}
                     </button>
                   ))}
                </div>
                
                <div className="p-4 rounded-lg bg-muted/50 font-mono text-xs border border-border/50 overflow-hidden">
                   <div className="flex items-center gap-2 text-muted-foreground">
                      <span className="text-green-600 dark:text-green-500 shrink-0">➜</span>
                      <span className="shrink-0">~</span>
                      <span className="opacity-50 truncate">config set log_level</span>
                   </div>
                   <div className="mt-2 pl-4 border-l-2 border-border/50">
                       <div className="flex gap-2 flex-wrap">
                           <span className="text-blue-600 dark:text-blue-500 shrink-0">current_value</span>
                           <span className="text-muted-foreground shrink-0">=</span> 
                           <span className={cn("font-bold", levelColors[logLevel].split(' ')[0])}>"{logLevel}"</span>
                       </div>
                       <div className="mt-1 text-muted-foreground/60 italic">
                           # {t('settings.restartRequired')}
                       </div>
                   </div>
                </div>
             </CardContent>
          </Card>

          {/* Log Folder Access */}
          <Card className="border-border shadow-sm bg-card relative overflow-hidden">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Terminal className="h-5 w-5 text-primary" />
                {t('settings.openLogFolder')}
              </CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
              <p className="text-sm text-muted-foreground max-w-xl">
                {t('settings.logFolderDescription')}
              </p>
              <Button
                variant="outline"
                className="self-start"
                onClick={handleOpenLogs}
                disabled={isOpeningLogs}
              >
                <Terminal className="h-4 w-4 mr-2" />
                {isOpeningLogs ? t('settings.opening') : t('settings.openLogFolder')}
              </Button>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
};

const AboutSettings = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string>('Loading...');

  useEffect(() => {
    // 获取应用版本
    invoke<string>('get_app_version')
      .then(setVersion)
      .catch(() => setVersion('Unknown'));
  }, []);

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div>
         <h2 className="text-2xl font-bold tracking-tight mb-6">{t('settings.about')}</h2>

         <div className="flex flex-col items-center justify-center py-12 bg-gradient-to-b from-primary/5 to-transparent rounded-3xl mb-8 border border-border/20">
            <h3 className="text-4xl font-bold text-foreground tracking-tight">NeuraDock</h3>
            <p className="text-muted-foreground font-mono mt-2">{version}</p>
         </div>

         <div className="max-w-3xl mx-auto space-y-6">
            <div className="grid gap-4">
               <div className="p-4 rounded-xl bg-muted/30 border border-border/50 flex justify-between items-center">
                  <span className="font-medium text-muted-foreground">{t('settings.copyright')}</span>
                  <span>© 2025 NeuraDock</span>
               </div>
            </div>

            {/* Disclaimer - Using raw div instead of Alert to prevent potential rendering issues */}
            <div className="rounded-2xl border border-amber-500/20 bg-amber-500/5 p-4 sm:p-6 flex gap-4">
              <div className="shrink-0 mt-1">
                <AlertTriangle className="h-5 w-5 text-amber-600 dark:text-amber-500" />
              </div>
              <div className="space-y-4 w-full">
                <div>
                  <div className="font-bold text-base text-amber-900 dark:text-amber-100">{t('disclaimer.title')}</div>
                  <div className="space-y-2 text-sm text-amber-900/80 dark:text-amber-100/80 leading-relaxed mt-2">
                    <p className="font-semibold">
                      {t('disclaimer.liability.title')}
                    </p>
                    <p>
                      {t('disclaimer.liability.description')}
                    </p>
                    <p className="font-semibold">
                      ⚠️ {t('disclaimer.liability.warning')}
                    </p>
                  </div>
                </div>

                <div className="pt-4 border-t border-amber-500/20">
                  <div className="flex items-center gap-2 font-semibold text-sm text-amber-900 dark:text-amber-100 mb-2">
                    <Scale className="h-4 w-4" />
                    {t('disclaimer.license.title')}
                  </div>
                  <div className="space-y-2 text-sm text-amber-900/80 dark:text-amber-100/80 leading-relaxed">
                    <p>{t('disclaimer.license.description')}</p>
                    <p className="font-semibold">{t('disclaimer.license.commercial')}</p>
                    <p className="italic opacity-80">
                      {t('disclaimer.license.footer')}
                    </p>
                  </div>
                </div>
              </div>
            </div>
         </div>
      </div>
    </div>
  );
};

// --- Main Page Component ---

export function SettingsPage() {
  const { t } = useTranslation();
  const [activeSection, setActiveSection] = useState<SettingSection>('appearance');

  const sidebarContent = (
    <Card className="flex-1 border-border/50 shadow-sm bg-background/50 backdrop-blur-sm overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-2 space-y-1">
          {navigationItems.map((item) => {
            const Icon = item.icon;
            const isActive = activeSection === item.id;

            return (
              <button
                key={item.id}
                onClick={() => setActiveSection(item.id)}
                className={cn(
                  "w-full flex items-center gap-3 px-3 py-2 rounded-lg text-left transition-colors",
                  isActive
                    ? "bg-primary text-primary-foreground shadow-sm"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground"
                )}
              >
                <Icon className="h-4 w-4 shrink-0" />
                <span className={cn(
                  "text-sm font-medium leading-none",
                  isActive && "font-semibold"
                )}>
                  {t(item.labelKey)}
                </span>
              </button>
            );
          })}
        </div>
      </ScrollArea>
    </Card>
  );

  const renderContent = () => {
    switch (activeSection) {
      case 'appearance': return <AppearanceSettings />;
      case 'performance': return <PerformanceSettings />;
      case 'notifications': return <NotificationSettings />;
      case 'developer': return <DeveloperSettings />;
      case 'about': return <AboutSettings />;
      default: return null;
    }
  };

  return (
    <PageContainer 
      className="h-full overflow-hidden"
      title={t('settings.title')}
    >
      <SidebarPageLayout sidebar={sidebarContent}>
        <div className="max-w-4xl mx-auto space-y-6 pb-20">
          {renderContent()}
        </div>
      </SidebarPageLayout>
    </PageContainer>
  );
}

