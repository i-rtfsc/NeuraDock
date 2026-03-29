import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Switch } from '@/components/ui/switch';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card } from '@/components/ui/card';
import { useTheme } from '@/hooks/useTheme';
import { useProxyConfig } from '@/hooks/useProxyConfig';
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
  Palette,
  Settings,
  HardDrive,
  Globe,
  ShieldCheck,
  FileText,
  CalendarClock
} from 'lucide-react';
import { useState, useEffect, ReactNode } from 'react';
import { toast } from 'sonner';
import { invoke } from '@tauri-apps/api/core';
import { NotificationChannelList } from '@/components/notification/NotificationChannelList';
import { useNotificationChannels } from '@/hooks/useNotificationChannels';
import { usePersistedState } from '@/hooks/usePersistedState';
import { cn } from '@/lib/utils';
import { PageContainer } from '@/components/layout/PageContainer';
import { PageContent } from '@/components/layout/PageContent';
import type { ThemeStyle, ThemeSkin } from '@/hooks/useTheme';

type PreferencesTab = 'appearance' | 'workspace' | 'system' | 'about';

const DEFAULT_PREFERENCES_TAB: PreferencesTab = 'appearance';

const STYLE_SWATCHES: Record<ThemeStyle, { primary: string; accent: string; neutral: string }> = {
  default: { primary: 'hsl(231 83% 58%)', accent: 'hsl(248 82% 66%)', neutral: 'hsl(222 30% 92%)' },
  graphite: { primary: 'hsl(222 16% 32%)', accent: 'hsl(216 14% 46%)', neutral: 'hsl(220 16% 88%)' },
  emerald: { primary: 'hsl(160 70% 36%)', accent: 'hsl(176 64% 38%)', neutral: 'hsl(156 24% 90%)' },
  sunset: { primary: 'hsl(20 87% 50%)', accent: 'hsl(35 86% 52%)', neutral: 'hsl(32 42% 90%)' },
  midnight: { primary: 'hsl(226 88% 50%)', accent: 'hsl(203 88% 50%)', neutral: 'hsl(224 26% 88%)' },
  ocean: { primary: 'hsl(197 90% 44%)', accent: 'hsl(214 84% 54%)', neutral: 'hsl(203 34% 90%)' },
  violet: { primary: 'hsl(258 72% 54%)', accent: 'hsl(280 66% 60%)', neutral: 'hsl(262 32% 90%)' },
};

const SKIN_PREVIEW_CLASS: Record<ThemeSkin, string> = {
  soft: 'rounded-2xl border border-border/40 shadow-sm bg-card p-2.5',
  pill: 'rounded-[1.35rem] border border-primary/35 shadow-sm bg-primary/10 p-2.5',
  sharp: 'rounded-sm border border-border/70 shadow-none bg-card p-2.5',
  glass: 'rounded-xl border border-border/45 shadow-md bg-background/60 backdrop-blur-md p-2.5',
  prism: 'rounded-[0.5rem] border border-primary/45 shadow-md bg-primary/10 p-2.5',
  hud: 'rounded-md border border-primary/45 shadow-[0_0_0_1px_hsl(var(--ring)/0.35),0_12px_24px_-14px_hsl(var(--ring)/0.55)] bg-background/60 backdrop-blur-md p-2.5',
  cyber: 'rounded-[0.42rem] border border-primary/55 shadow-[0_0_0_1px_hsl(var(--ring)/0.45),0_0_24px_hsl(var(--ring)/0.45)] bg-background/70 backdrop-blur-sm p-2.5',
  'cyber-darkline': 'rounded-[0.38rem] border border-primary/60 shadow-[0_0_0_1px_hsl(var(--ring)/0.55),0_0_22px_hsl(var(--ring)/0.52),inset_0_0_0_1px_hsl(var(--ring)/0.2)] bg-background/80 backdrop-blur-sm p-2.5',
};

const skinLabelKey = (option: ThemeSkin) =>
  `settings.themeSkin${option
    .split('-')
    .map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
    .join('')}`;

function normalizePreferencesTab(value?: string | null): PreferencesTab {
  switch (value) {
    case 'appearance':
    case 'workspace':
    case 'system':
    case 'about':
      return value;
    case 'general':
      return 'appearance';
    case 'notifications':
      return 'workspace';
    default:
      return DEFAULT_PREFERENCES_TAB;
  }
}

// --- Reusable Layout Components ---

interface SettingsGroupProps {
  title?: string;
  children: ReactNode;
  className?: string;
  contentClassName?: string;
}

const SettingsGroup = ({ title, children, className, contentClassName }: SettingsGroupProps) => (
  <div className={cn("space-y-3 w-full", className)}>
    {title && (
      <div className="flex items-center gap-2 px-1 ml-1">
        <Badge variant="soft-primary" className="rounded-md h-6 font-bold tracking-wider text-[10px]">
          {title}
        </Badge>
      </div>
    )}
    <Card className={cn(
      "card-vivid group p-0 overflow-hidden cursor-default",
      contentClassName
    )}>
      <div className="flex flex-col w-full">
        {children}
      </div>
    </Card>
  </div>
);

interface SettingsRowProps {
  icon?: React.ElementType;
  label: string;
  description?: string;
  children?: ReactNode;
  childrenPlacement?: 'inline' | 'below';
  onClick?: () => void;
  className?: string;
  childrenContainerClassName?: string;
  action?: ReactNode;
  isLast?: boolean;
}

const SettingsRow = ({
  icon: Icon,
  label,
  description,
  children,
  childrenPlacement = 'inline',
  onClick,
  className,
  childrenContainerClassName,
  action,
  isLast,
}: SettingsRowProps) => {
  const hasInlineChildren = Boolean(children) && childrenPlacement === 'inline';
  const hasBelowChildren = Boolean(children) && childrenPlacement === 'below';
  const showTrailing = hasInlineChildren || Boolean(action) || (Boolean(onClick) && !action && !hasInlineChildren);

  return (
    <div className="relative w-full">
      <div
        className={cn(
          "flex items-center gap-4 px-6 py-5 min-h-[4.5rem] transition-all duration-base ease-smooth hover:bg-primary/[0.03] w-full",
          onClick && "cursor-pointer active:bg-primary/5",
          className
        )}
        onClick={onClick}
      >
        {Icon && (
          <div className="flex items-center justify-center w-10 h-10 rounded-xl bg-background border border-border/60 shadow-sm text-muted-foreground group-hover:text-primary group-hover:border-primary/20 transition-colors duration-base ease-smooth shrink-0">
            <Icon className="h-5 w-5" />
          </div>
        )}
        <div className="flex-1 min-w-0 flex flex-col justify-center">
          <Label className={cn("text-base font-bold tracking-tight text-foreground", onClick && "cursor-pointer")}>
            {label}
          </Label>
          {description && <p className="text-sm text-muted-foreground mt-1 leading-normal font-medium opacity-80">{description}</p>}
        </div>
        {showTrailing && (
          <div className={cn('shrink-0 flex items-center gap-3 pl-4', childrenContainerClassName)}>
            {hasInlineChildren ? children : null}
            {action}
            {onClick && !action && !hasInlineChildren && (
              <ChevronRight className="h-5 w-5 text-muted-foreground/40 group-hover:text-primary transition-colors" />
            )}
          </div>
        )}
      </div>
      {hasBelowChildren && (
        <div className={cn('w-full px-6 pb-5 pt-1', Icon && 'pl-[5rem]', childrenContainerClassName)}>
          {children}
        </div>
      )}
      {!isLast && (
        <div className="mx-6 h-px bg-border/40" />
      )}
    </div>
  );
};

const ThemeStyleCard = ({
  option,
  selected,
  onSelect,
}: {
  option: ThemeStyle;
  selected: boolean;
  onSelect: (value: ThemeStyle) => void;
}) => {
  const { t } = useTranslation();
  const swatch = STYLE_SWATCHES[option];

  return (
    <button
      type="button"
      onClick={() => onSelect(option)}
      className={cn(
        'group w-full rounded-[var(--radius-control)] border bg-background px-2.5 py-2 text-left transition-all duration-base ease-smooth',
        selected
          ? 'border-primary/55 ring-2 ring-primary/20 shadow-hover-sm'
          : 'border-border/50 hover:border-primary/35 hover:shadow-sm'
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="truncate text-xs font-semibold text-foreground">
          {t(`settings.theme${option.charAt(0).toUpperCase()}${option.slice(1)}`)}
        </span>
        {selected && (
          <Badge variant="soft-primary" className="h-4 px-1.5 text-[9px]">
            {t('common.selected')}
          </Badge>
        )}
      </div>

      <div className="mt-2 flex items-center gap-1.5">
        <span className="h-3 w-3 rounded-full border border-white/60 shadow-sm" style={{ backgroundColor: swatch.primary }} />
        <span className="h-3 w-3 rounded-full border border-white/60 shadow-sm" style={{ backgroundColor: swatch.accent }} />
        <span className="h-3 w-3 rounded-full border border-white/60 shadow-sm" style={{ backgroundColor: swatch.neutral }} />
        <div
          className="ml-auto h-4 w-12 rounded-full border border-border/50"
          style={{ backgroundImage: `linear-gradient(135deg, ${swatch.primary} 0%, ${swatch.accent} 100%)` }}
        />
      </div>
    </button>
  );
};

const ThemeSkinCard = ({
  option,
  selected,
  onSelect,
}: {
  option: ThemeSkin;
  selected: boolean;
  onSelect: (value: ThemeSkin) => void;
}) => {
  const { t } = useTranslation();

  return (
    <button
      type="button"
      onClick={() => onSelect(option)}
      className={cn(
        'group w-full rounded-[var(--radius-control)] border bg-background px-2.5 py-2 text-left transition-all duration-base ease-smooth',
        selected
          ? 'border-primary/55 ring-2 ring-primary/20 shadow-hover-sm'
          : 'border-border/50 hover:border-primary/35 hover:shadow-sm'
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="truncate text-xs font-semibold text-foreground">
          {t(skinLabelKey(option))}
        </span>
        {selected && (
          <Badge variant="soft-primary" className="h-4 px-1.5 text-[9px]">
            {t('common.selected')}
          </Badge>
        )}
      </div>

      <div className="mt-2">
        <div className={SKIN_PREVIEW_CLASS[option]}>
          <div className="mb-1.5 h-2 w-2/3 rounded bg-muted" />
          <div className="flex items-center gap-1.5">
            <div className="h-2 w-8 rounded bg-primary/60" />
            <div className="h-2 flex-1 rounded bg-muted" />
          </div>
        </div>
      </div>
    </button>
  );
};

// --- Sub-Components ---

const GeneralSettings = () => {
  const { theme, setTheme, style, setThemeStyle, skin, setThemeSkin } = useTheme();
  const { t, i18n } = useTranslation();
  const THEME_STYLE_OPTIONS: ThemeStyle[] = [
    'default',
    'graphite',
    'emerald',
    'sunset',
    'midnight',
    'ocean',
    'violet',
  ];
  const THEME_SKIN_OPTIONS: ThemeSkin[] = [
    'soft',
    'pill',
    'sharp',
    'glass',
    'prism',
    'hud',
    'cyber',
    'cyber-darkline',
  ];

  const handleLanguageChange = (lang: string) => {
    i18n.changeLanguage(lang);
    localStorage.setItem('language', lang);
    toast.success(t('common.success'));
  };

  return (
    <div className="space-y-section-gap animate-in fade-in duration-base w-full">
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
          icon={Palette}
          label={t('settings.themeStyle')}
          description={t('settings.themeStyleDescription')}
          childrenPlacement="below"
          childrenContainerClassName="w-full pt-3"
        >
          <div className="w-full">
            <div className="grid w-full grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
              {THEME_STYLE_OPTIONS.map((styleOption) => (
                <ThemeStyleCard
                  key={styleOption}
                  option={styleOption}
                  selected={style === styleOption}
                  onSelect={setThemeStyle}
                />
              ))}
            </div>
          </div>
        </SettingsRow>

        <SettingsRow
          icon={Palette}
          label={t('settings.themeSkin')}
          description={t('settings.themeSkinDescription')}
          childrenPlacement="below"
          childrenContainerClassName="w-full pt-3"
        >
          <div className="w-full">
            <div className="grid w-full grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
              {THEME_SKIN_OPTIONS.map((skinOption) => (
                <ThemeSkinCard
                  key={skinOption}
                  option={skinOption}
                  selected={skin === skinOption}
                  onSelect={setThemeSkin}
                />
              ))}
            </div>
          </div>
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
  const { config: proxyConfig, isLoading: proxyLoading, isSaving: proxySaving, updateField, saveConfig } = useProxyConfig();

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

  const handleSaveProxy = async () => {
    // Validate before saving
    if (proxyConfig.enabled && (!proxyConfig.host || proxyConfig.port === 0)) {
      toast.error(t('settings.proxyValidationError'));
      return;
    }
    await saveConfig(proxyConfig);
  };

  const handleProxyEnabledChange = async (checked: boolean) => {
    updateField('enabled', checked);
    if (!checked) {
      await saveConfig({ ...proxyConfig, enabled: false });
    }
  };

  return (
    <div className="space-y-section-gap animate-in fade-in duration-base w-full">
      {/* Cache Control */}
      <SettingsGroup title={t('settings.cacheControl')}>
        <div className="p-5 space-y-6 group">
           <div className="flex items-start justify-between">
               <div className="flex gap-4">
                    <div className="flex items-center justify-center w-10 h-10 rounded-[var(--radius-control-lg)] bg-primary/10 text-primary border border-primary/20 shadow-sm shrink-0">
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

           <div className="pt-2 transition-opacity duration-slow ease-smooth">
               <input
                   type="range"
                   min="1"
                   max="24"
                   step="1"
                   value={cacheAgeHours}
                   onChange={(e) => handleSaveCache(parseInt(e.target.value))}
                   className="w-full h-2 bg-muted rounded-full appearance-none cursor-pointer accent-primary focus:outline-none focus:ring-2 focus:ring-primary/20 transition-all duration-base ease-smooth"
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

      {/* Network & Proxy */}
      <SettingsGroup title={t('settings.network')}>
        <div className="p-6 space-y-6">
          {/* Proxy Enable/Disable */}
          <div className="flex items-start justify-between">
            <div className="flex gap-4">
              <div className="flex items-center justify-center w-10 h-10 rounded-[var(--radius-control-lg)] bg-primary/10 text-primary border border-primary/20 shadow-sm shrink-0">
                <Globe className="h-5 w-5" />
              </div>
              <div className="space-y-1">
                <Label className="text-base font-medium text-foreground">
                  {t('settings.proxyEnabled')}
                </Label>
                <p className="text-sm text-muted-foreground leading-snug max-w-[280px] md:max-w-md">
                  {t('settings.proxyDescription')}
                </p>
              </div>
            </div>
            <Switch
              checked={proxyConfig.enabled}
              onCheckedChange={handleProxyEnabledChange}
              disabled={proxyLoading}
            />
          </div>

          {/* Proxy Configuration */}
          {proxyConfig.enabled && (
            <div className="space-y-4 pl-14 pt-2 border-l-2 border-primary/20 animate-in fade-in duration-base">
              {/* Proxy Type */}
              <div className="space-y-2">
                <Label className="text-sm font-medium text-foreground">
                  {t('settings.proxyType')}
                </Label>
                <Select
                  value={proxyConfig.proxy_type}
                  onValueChange={(value) => updateField('proxy_type', value)}
                  disabled={proxyLoading}
                >
                  <SelectTrigger className="w-full h-input text-sm border-border/50 bg-background/50">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="http">{t('settings.proxyTypeHttp')}</SelectItem>
                    <SelectItem value="socks5">{t('settings.proxyTypeSocks5')}</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  {proxyConfig.proxy_type === 'http'
                    ? t('settings.proxyTypeHttpHint')
                    : t('settings.proxyTypeSocks5Hint')}
                </p>
              </div>

              {/* Host */}
              <div className="space-y-2">
                <Label htmlFor="proxy-host" className="text-sm font-medium text-foreground">
                  {t('settings.proxyHost')}
                </Label>
                <Input
                  id="proxy-host"
                  type="text"
                  placeholder="127.0.0.1"
                  value={proxyConfig.host}
                  onChange={(e) => updateField('host', e.target.value)}
                  disabled={proxyLoading}
                  className="h-input text-sm"
                />
              </div>

              {/* Port */}
              <div className="space-y-2">
                <Label htmlFor="proxy-port" className="text-sm font-medium text-foreground">
                  {t('settings.proxyPort')}
                </Label>
                <Input
                  id="proxy-port"
                  type="number"
                  placeholder="7890"
                  min="1"
                  max="65535"
                  value={proxyConfig.port || ''}
                  onChange={(e) => updateField('port', parseInt(e.target.value) || 0)}
                  disabled={proxyLoading}
                  className="h-input text-sm"
                />
              </div>

              {/* Example */}
              {proxyConfig.host && proxyConfig.port > 0 && (
                <div className="text-xs text-muted-foreground bg-muted/30 px-3 py-2 rounded-[var(--radius-control)] border border-border/30 font-mono">
                  {proxyConfig.proxy_type}://{proxyConfig.host}:{proxyConfig.port}
                </div>
              )}

              {/* Save Button */}
              <Button
                onClick={handleSaveProxy}
                disabled={proxySaving || proxyLoading || !proxyConfig.host || proxyConfig.port === 0}
                className="w-full h-input"
              >
                {proxySaving ? t('common.saving', { defaultValue: 'Saving...' }) : t('common.save', { defaultValue: 'Save' })}
              </Button>
            </div>
          )}
        </div>
      </SettingsGroup>

      {/* Storage */}
      <SettingsGroup title={t('settings.storageTitle')}>
        <SettingsRow 
          icon={HardDrive}
          label={t('settings.localDatabase')}
          description={t('settings.localDatabaseDescription')}
          action={<span className="text-xs font-semibold text-foreground bg-muted/60 px-3 py-1.5 rounded-[var(--radius-control)] border border-border/40 tabular-nums">12.5 MB</span>}
        />
        <SettingsRow 
           icon={Trash2}
           label={t('settings.temporaryFiles')}
           description={t('settings.temporaryFilesDescription')}
           action={<span className="text-xs font-semibold text-muted-foreground bg-muted/30 px-3 py-1.5 rounded-[var(--radius-control)] border border-border/30">{t('settings.empty')}</span>}
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

const WorkspaceSettings = () => {
  const { t } = useTranslation();
  const { data: notificationChannels = [], refetch: refetchChannels } = useNotificationChannels();

  return (
    <div className="space-y-section-gap animate-in fade-in duration-base w-full">
      <SettingsGroup
        title={t('settings.notification')}
        contentClassName="p-4 sm:p-5"
      >
        <NotificationChannelList
          channels={notificationChannels}
          onUpdate={refetchChannels}
        />
      </SettingsGroup>
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
  const profileColorClass = profileText === 'Debug'
    ? 'bg-warning-soft text-warning border border-warning-border'
    : 'bg-success-soft text-success border border-success-border';

  return (
    <div className="space-y-section-gap w-full">
      <SettingsGroup title={t('settings.about')}>
        <div className="p-6 sm:p-7 border-b border-border/40">
          <div className="grid gap-4 sm:grid-cols-2">
            <Card className="border-border/40 bg-muted/20 p-4">
              <div className="flex items-start gap-3">
                <div className="h-9 w-9 rounded-[var(--radius-control)] bg-primary/10 text-primary border border-primary/20 flex items-center justify-center">
                  <ShieldCheck className="h-4 w-4" />
                </div>
                <div className="space-y-1 min-w-0">
                  <p className="text-xs text-muted-foreground">{t('settings.version')}</p>
                  <p className="text-base font-semibold text-foreground font-mono truncate">v{appVersion.version}</p>
                </div>
              </div>
            </Card>
            <Card className="border-border/40 bg-muted/20 p-4">
              <div className="flex items-start gap-3">
                <div className="h-9 w-9 rounded-[var(--radius-control)] bg-info-soft text-info border border-info-border flex items-center justify-center">
                  <CalendarClock className="h-4 w-4" />
                </div>
                <div className="space-y-1 min-w-0">
                  <p className="text-xs text-muted-foreground">{t('settings.buildProfile')}</p>
                  <span className={cn("inline-flex px-2 py-0.5 rounded-full text-[10px] font-bold tracking-wider", profileColorClass)}>
                    {profileText}
                  </span>
                </div>
              </div>
            </Card>
          </div>
        </div>

        <SettingsRow
          icon={Info}
          label={t('settings.copyright')}
          action={<span className="text-sm font-medium text-muted-foreground">© 2026 NeuraDock</span>}
        />
        <SettingsRow
          icon={FileText}
          label={t('disclaimer.license.title')}
          description={t('disclaimer.license.description')}
          isLast
        />
      </SettingsGroup>

      <SettingsGroup title={t('disclaimer.title')}>
        <div className="p-5 sm:p-6 grid gap-4">
          <Card className="border-warning-border bg-warning-soft/40 p-4">
            <div className="flex items-start gap-3">
              <AlertTriangle className="h-4 w-4 mt-0.5 text-warning shrink-0" />
              <div className="space-y-1.5">
                <h4 className="text-sm font-semibold text-foreground">{t('disclaimer.liability.title')}</h4>
                <p className="text-xs leading-relaxed text-muted-foreground">{t('disclaimer.liability.description')}</p>
                <p className="text-xs font-medium text-warning-soft-foreground">{t('disclaimer.liability.warning')}</p>
              </div>
            </div>
          </Card>

          <Card className="border-info-border bg-info-soft/35 p-4">
            <div className="flex items-start gap-3">
              <Scale className="h-4 w-4 mt-0.5 text-info shrink-0" />
              <div className="space-y-1.5">
                <h4 className="text-sm font-semibold text-foreground">{t('disclaimer.license.title')}</h4>
                <p className="text-xs leading-relaxed text-muted-foreground">{t('disclaimer.license.commercial')}</p>
                <p className="text-xs italic text-muted-foreground/80">{t('disclaimer.license.footer')}</p>
              </div>
            </div>
          </Card>
        </div>
      </SettingsGroup>
    </div>
  );
};

// --- Main Page Component ---

export function PreferencesPage() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = usePersistedState<PreferencesTab>(
    'page-tab:preferences',
    DEFAULT_PREFERENCES_TAB
  );

  return (
    <Tabs
      value={normalizePreferencesTab(activeTab)}
      onValueChange={(value) => setActiveTab(normalizePreferencesTab(value))}
      className="h-full flex flex-col w-full bg-background/95"
    >
      <PageContainer 
        className="h-full bg-muted/10 w-full" 
        title={t('settings.title')}
        actions={
          <TabsList className="h-10 bg-muted/50 border border-border/50 p-1 rounded-[var(--radius-control)] inline-flex items-center justify-center">
            <TabsTrigger 
              value="appearance"
              className="text-sm font-medium px-4 h-8 rounded-md data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm transition-all duration-base ease-smooth"
            >
              <Settings className="w-4 h-4 mr-2" />
              {t('settings.appearance', 'Appearance')}
            </TabsTrigger>
            <TabsTrigger
              value="workspace"
              className="text-sm font-medium px-4 h-8 rounded-md data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm transition-all duration-base ease-smooth"
            >
              {t('settings.workspace', 'Workspace')}
            </TabsTrigger>
            <TabsTrigger 
              value="system" 
              className="text-sm font-medium px-4 h-8 rounded-md data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm transition-all duration-base ease-smooth"
            >
              <HardDrive className="w-4 h-4 mr-2" />
              {t('settings.system', 'System')}
            </TabsTrigger>
            <TabsTrigger 
              value="about" 
              className="text-sm font-medium px-4 h-8 rounded-md data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm transition-all duration-base ease-smooth"
            >
              {t('settings.about')}
            </TabsTrigger>
          </TabsList>
        }
      >
        <PageContent maxWidth="none" className="h-full page-enter-stagger">
             <div className="pb-32 w-full">
                <TabsContent value="appearance" className="mt-0 outline-none w-full">
                  <GeneralSettings />
                </TabsContent>
                <TabsContent value="workspace" className="mt-0 outline-none w-full">
                  <WorkspaceSettings />
                </TabsContent>
                <TabsContent value="system" className="mt-0 outline-none w-full">
                  <SystemSettings />
                </TabsContent>
                <TabsContent value="about" className="mt-0 outline-none w-full">
                  <AboutSettings />
                </TabsContent>
             </div>
        </PageContent>
      </PageContainer>
    </Tabs>
  );
}
