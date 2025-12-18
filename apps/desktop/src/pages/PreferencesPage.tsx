import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useTheme } from '@/hooks/useTheme';
import { useTranslation } from 'react-i18next';
import {
  Info, Sun,
  Database, Trash2,
  Terminal,
  FolderOpen,
  ChevronRight,
  Languages,
  AlertTriangle,
  Scale,
  Settings,
  HardDrive
} from 'lucide-react';
import { useState, useEffect, ReactNode } from 'react';
import { toast } from 'sonner';
import { invoke } from '@tauri-apps/api/core';
import { NotificationChannelList } from '@/components/notification/NotificationChannelList';
import { useNotificationChannels } from '@/hooks/useNotificationChannels';
import { cn } from '@/lib/utils';
import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';

// --- Reusable Layout Components ---

interface SettingsGroupProps {
  title?: string;
  children: ReactNode;
  className?: string;
  contentClassName?: string;
}

const SettingsGroup = ({ title, children, className, contentClassName }: SettingsGroupProps) => (
  <div className={cn("space-y-element-gap w-full", className)}>
    {title && (
      <h3 className="text-sm font-semibold text-foreground/70 px-1 ml-1 uppercase tracking-wide">
        {title}
      </h3>
    )}
    <div className={cn(
      "bg-card/50 backdrop-blur-md border border-border/60 rounded-2xl overflow-hidden shadow-sm w-full transition-all duration-200 hover:shadow-md hover:scale-[1.005] hover:border-border/80",
      contentClassName
    )}>
      <div className="flex flex-col w-full">
        {children}
      </div>
    </div>
  </div>
);

interface SettingsRowProps {
  icon?: React.ElementType;
  label: string;
  description?: string;
  children?: ReactNode;
  onClick?: () => void;
  className?: string;
  action?: ReactNode;
  isLast?: boolean;
}

const SettingsRow = ({ icon: Icon, label, description, children, onClick, className, action, isLast }: SettingsRowProps) => (
  <div className="relative group w-full">
    <div
      className={cn(
        "flex items-center gap-element-gap px-[var(--layout-page-content-padding)] py-5 min-h-[4rem] transition-all duration-200 hover:bg-muted/30 w-full",
        onClick && "cursor-pointer active:bg-muted/50 active:scale-[0.99]",
        className
      )}
      onClick={onClick}
    >
      {Icon && (
        <div className="flex items-center justify-center w-10 h-10 rounded-xl bg-background border border-border/60 shadow-sm text-muted-foreground group-hover:text-primary group-hover:border-primary/20 transition-all shrink-0">
          <Icon className="h-5 w-5" />
        </div>
      )}
      <div className="flex-1 min-w-0 flex flex-col justify-center">
        <Label className={cn("text-base font-medium text-foreground", onClick && "cursor-pointer")}>
          {label}
        </Label>
        {description && <p className="text-sm text-muted-foreground mt-1 leading-normal">{description}</p>}
      </div>
      <div className="shrink-0 flex items-center gap-element-gap pl-4">
        {children}
        {action}
        {onClick && !action && !children && <ChevronRight className="h-5 w-5 text-muted-foreground/40" />}
      </div>
    </div>
    {!isLast && (
      <div className="absolute bottom-0 left-20 right-0 h-px bg-border/40" />
    )}
  </div>
);

// --- Sub-Components ---

const GeneralSettings = () => {
  const { theme, setTheme } = useTheme();
  const { t, i18n } = useTranslation();

  const handleLanguageChange = (lang: string) => {
    i18n.changeLanguage(lang);
    localStorage.setItem('language', lang);
    toast.success(t('common.success'));
  };

  return (
    <div className="space-y-section-gap animate-in fade-in slide-in-from-bottom-2 duration-500 w-full">
      <SettingsGroup title={t('settings.appearance', { defaultValue: 'Appearance' })}>
        <SettingsRow 
          icon={Sun}
          label={t('settings.theme')}
          description={t('settings.appearanceThemeDescription')}
        >
          <Select value={theme} onValueChange={(value) => setTheme(value as 'light' | 'dark' | 'system')}>
            <SelectTrigger className="w-[160px] h-input-sm text-sm border-border/50 bg-background/50 focus:ring-primary/20">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="light">{t('settings.light')}</SelectItem>
              <SelectItem value="dark">{t('settings.dark')}</SelectItem>
              <SelectItem value="system">{t('settings.system')}</SelectItem>
            </SelectContent>
          </Select>
        </SettingsRow>

        <SettingsRow
          icon={Languages}
          label={t('settings.language')}
          description={t('settings.appearanceLanguageDescription')}
          isLast
        >
          <Select value={i18n.language} onValueChange={handleLanguageChange}>
            <SelectTrigger className="w-[160px] h-input-sm text-sm border-border/50 bg-background/50 focus:ring-primary/20">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="zh-CN">🇨🇳 简体中文</SelectItem>
              <SelectItem value="en-US">🇺🇸 English</SelectItem>
            </SelectContent>
          </Select>
        </SettingsRow>
      </SettingsGroup>
    </div>
  );
};

const SystemSettings = () => {
  const { t } = useTranslation();
  const [cacheAgeHours, setCacheAgeHours] = useState<number>(1);
  const [logLevel, setLogLevel] = useState<string>('info');

  useEffect(() => {
    const stored = localStorage.getItem('maxCacheAgeHours');
    if (stored) {
      setCacheAgeHours(parseInt(stored, 10));
    }
    invoke<string>('get_log_level').then(setLogLevel).catch(console.error);
  }, []);

  const handleSaveCache = (val: number) => {
    setCacheAgeHours(val);
    localStorage.setItem('maxCacheAgeHours', val.toString());
  };

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
      const logPath = await invoke<string>('open_log_dir');
      toast.success(t('settings.logFolderOpened', { path: logPath }));
    } catch (error) {
      toast.error(t('settings.failedToOpenLogs') + ': ' + String(error));
    }
  };

  return (
    <div className="space-y-section-gap animate-in fade-in slide-in-from-bottom-2 duration-500 w-full">
      {/* Cache Control */}
      <SettingsGroup title={t('settings.cacheControl')}>
        <div className="p-5 space-y-6 group">
           <div className="flex items-start justify-between">
               <div className="flex gap-4">
                    <div className="flex items-center justify-center w-10 h-10 rounded-xl bg-primary/10 text-primary border border-primary/20 shadow-sm shrink-0">
                       <Database className="h-5 w-5" />
                    </div>
                    <div className="space-y-1">
                       <Label className="text-base font-medium text-foreground">
                           {t('settings.cacheAge')}
                       </Label>
                       <p className="text-sm text-muted-foreground leading-snug max-w-[280px] md:max-w-md">
                           {t('settings.cacheAgeDescription')}
                       </p>
                    </div>
               </div>
               
               <div className="flex flex-col items-end shrink-0">
                    <div className="flex items-baseline gap-1">
                       <span className="text-2xl font-bold tabular-nums text-primary">{cacheAgeHours}</span>
                       <span className="text-sm font-medium text-muted-foreground">{t('settings.hours')}</span>
                    </div>
               </div>
           </div>

           <div className="pt-2 transition-opacity duration-300">
               <input
                   type="range"
                   min="1"
                   max="24"
                   step="1"
                   value={cacheAgeHours}
                   onChange={(e) => handleSaveCache(parseInt(e.target.value))}
                   className="w-full h-2 bg-muted rounded-full appearance-none cursor-pointer accent-primary focus:outline-none focus:ring-2 focus:ring-primary/20 transition-all"
               />
               <div className="relative mt-3 h-4 text-xs font-medium text-muted-foreground/50 select-none">
                   <span className="absolute left-[0%] -translate-x-1/2">1h</span>
                   <span className="absolute left-[calc(5/23*100%)] -translate-x-1/2">6h</span>
                   <span className="absolute left-[calc(11/23*100%)] -translate-x-1/2">12h</span>
                   <span className="absolute left-[calc(17/23*100%)] -translate-x-1/2">18h</span>
                   <span className="absolute left-[100%] -translate-x-1/2">24h</span>
               </div>
           </div>
        </div>
      </SettingsGroup>

      {/* Storage */}
      <SettingsGroup title={t('settings.storageTitle')}>
        <SettingsRow 
          icon={HardDrive}
          label={t('settings.localDatabase')}
          description={t('settings.localDatabaseDescription')}
          action={<span className="text-xs font-semibold text-foreground bg-muted/60 px-3 py-1.5 rounded-lg border border-border/40 tabular-nums">12.5 MB</span>}
        />
        <SettingsRow 
           icon={Trash2}
           label={t('settings.temporaryFiles')}
           description={t('settings.temporaryFilesDescription')}
           action={<span className="text-xs font-semibold text-muted-foreground bg-muted/30 px-3 py-1.5 rounded-lg border border-border/30">{t('settings.empty')}</span>}
           isLast
        />
      </SettingsGroup>

      {/* Developer */}
      <SettingsGroup title={t('settings.developer')}>
        <SettingsRow 
          icon={Terminal}
          label={t('settings.logLevel')}
          description={t('settings.restartRequired')}
        >
           <Select value={logLevel} onValueChange={handleLogLevelChange}>
             <SelectTrigger className="w-[140px] h-input text-sm font-mono border-border/50 bg-background/50">
               <SelectValue />
             </SelectTrigger>
             <SelectContent>
               {['error', 'warn', 'info', 'debug', 'trace'].map(level => (
                 <SelectItem key={level} value={level} className="font-mono text-xs">
                   {level.toUpperCase()}
                 </SelectItem>
               ))}
             </SelectContent>
           </Select>
        </SettingsRow>

        <SettingsRow 
          icon={FolderOpen}
          label={t('settings.openLogFolder')}
          description={t('settings.logFolderDescription')}
          onClick={handleOpenLogs}
          isLast
        />
      </SettingsGroup>
    </div>
  );
};

const NotificationSettings = () => {
  const { data: notificationChannels = [], refetch: refetchChannels } = useNotificationChannels();

  return (
    <div className="animate-in fade-in slide-in-from-bottom-2 duration-500 w-full">
      <NotificationChannelList
        channels={notificationChannels}
        onUpdate={refetchChannels}
      />
    </div>
  );
};

const AboutSettings = () => {
  const { t } = useTranslation();
  const [appVersion, setAppVersion] = useState<{ version: string; profile: string }>({ version: 'Loading...', profile: 'Unknown' });

  useEffect(() => {
    invoke<string>('get_app_version')
      .then(fullVersion => {
        const match = fullVersion.match(/^(.*) \((.*)\)$/);
        if (match && match.length === 3) {
          setAppVersion({ version: match[1], profile: match[2] });
        } else {
          setAppVersion({ version: fullVersion, profile: 'Unknown' });
        }
      })
      .catch(() => setAppVersion({ version: 'Unknown', profile: 'Unknown' }));
  }, []);

  const profileText = appVersion.profile;
  const profileColorClass = profileText === 'Debug' ? 'bg-orange-500/10 text-orange-600 dark:text-orange-400 border-orange-500/20' : 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20';

  return (
    <div className="space-y-section-gap animate-in fade-in slide-in-from-bottom-2 duration-500 w-full">
      {/* App Info Group */}
      <SettingsGroup title={t('settings.about')}>
        <div className="p-8 flex flex-col items-center text-center gap-5 border-b border-border/40 bg-gradient-to-b from-muted/20 to-transparent">
           <div className="w-20 h-20 rounded-[1.5rem] bg-gradient-to-br from-primary to-primary/80 flex items-center justify-center text-primary-foreground shadow-lg shadow-primary/20">
              <span className="text-4xl font-bold">N</span>
           </div>
           
           <div className="space-y-1.5">
              <h3 className="text-2xl font-bold tracking-tight text-foreground">NeuraDock</h3>
              <div className="flex items-center gap-2 justify-center">
                 <span className="text-muted-foreground font-mono text-sm">v{appVersion.version}</span>
                 <span className={cn("px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider", profileColorClass)}>
                   {profileText}
                 </span>
              </div>
           </div>
        </div>
        
        <SettingsRow 
           icon={Info} 
           label={t('settings.copyright')} 
           action={<span className="text-sm font-medium text-muted-foreground">© 2025 NeuraDock</span>}
           isLast
        />
      </SettingsGroup>

      {/* Legal & Disclaimer Group */}
      <SettingsGroup title={t('disclaimer.title')}>
         <div className="p-6 space-y-8">
            {/* Liability Section */}
            <div className="flex gap-5 items-start">
               <div className="shrink-0 w-10 h-10 rounded-xl bg-amber-500/10 flex items-center justify-center text-amber-600 border border-amber-500/20">
                  <AlertTriangle className="h-5 w-5" />
               </div>
               <div className="space-y-3 flex-1">
                  <h4 className="text-base font-semibold text-foreground">{t('disclaimer.liability.title')}</h4>
                  <p className="text-sm text-muted-foreground leading-relaxed">
                    {t('disclaimer.liability.description')}
                  </p>
                  <div className="text-xs font-medium text-amber-600 dark:text-amber-500 bg-amber-500/5 px-3 py-2 rounded-lg border border-amber-500/10">
                    ⚠️ {t('disclaimer.liability.warning')}
                  </div>
               </div>
            </div>

            <div className="w-full h-px bg-border/40" />

            {/* License Section */}
            <div className="flex gap-5 items-start">
               <div className="shrink-0 w-10 h-10 rounded-xl bg-blue-500/10 flex items-center justify-center text-blue-600 border border-blue-500/20">
                  <Scale className="h-5 w-5" />
               </div>
               <div className="space-y-3 flex-1">
                  <h4 className="text-base font-semibold text-foreground">{t('disclaimer.license.title')}</h4>
                  <div className="text-sm text-muted-foreground leading-relaxed space-y-3">
                    <p>{t('disclaimer.license.description')}</p>
                    <p className="font-medium text-foreground/90 bg-muted/30 px-3 py-2 rounded-lg">{t('disclaimer.license.commercial')}</p>
                    <p className="text-xs italic opacity-70">{t('disclaimer.license.footer')}</p>
                  </div>
               </div>
            </div>
         </div>
      </SettingsGroup>
    </div>
  );
};

// --- Main Page Component ---

export function PreferencesPage() {
  const { t } = useTranslation();

  return (
    <Tabs defaultValue="general" className="h-full flex flex-col w-full bg-background/95">
      <PageContainer 
        className="h-full bg-muted/10 w-full" 
        title={t('settings.title')}
        actions={
          <TabsList className="h-10 bg-muted/50 border border-border/50 p-1 rounded-lg inline-flex items-center justify-center">
            <TabsTrigger 
              value="general" 
              className="text-sm font-medium px-4 h-8 rounded-md data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm transition-all"
            >
              <Settings className="w-4 h-4 mr-2" />
              {t('settings.general', 'General')}
            </TabsTrigger>
            <TabsTrigger 
              value="system" 
              className="text-sm font-medium px-4 h-8 rounded-md data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm transition-all"
            >
              <HardDrive className="w-4 h-4 mr-2" />
              {t('settings.system', 'System')}
            </TabsTrigger>
            <TabsTrigger 
              value="notifications" 
              className="text-sm font-medium px-4 h-8 rounded-md data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm transition-all"
            >
              {t('settings.notification')}
            </TabsTrigger>
            <TabsTrigger 
              value="about" 
              className="text-sm font-medium px-4 h-8 rounded-md data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm transition-all"
            >
              {t('settings.about')}
            </TabsTrigger>
          </TabsList>
        }
      >
        <PageContent maxWidth="lg" className="h-full">
          <ScrollArea className="h-full w-full rounded-2xl">
             <div className="pb-32 w-full">
                <TabsContent value="general" className="mt-0 outline-none w-full">
                  <GeneralSettings />
                </TabsContent>
                <TabsContent value="system" className="mt-0 outline-none w-full">
                  <SystemSettings />
                </TabsContent>
                <TabsContent value="notifications" className="mt-0 outline-none w-full">
                  <NotificationSettings />
                </TabsContent>
                <TabsContent value="about" className="mt-0 outline-none w-full">
                  <AboutSettings />
                </TabsContent>
             </div>
          </ScrollArea>
        </PageContent>
      </PageContainer>
    </Tabs>
  );
}
