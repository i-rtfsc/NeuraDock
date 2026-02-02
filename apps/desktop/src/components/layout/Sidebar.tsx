import { Link, useLocation } from 'react-router-dom';
import {
  Home,
  UserCircle,
  Key,
  Server,
  Settings,
  PanelLeftClose,
  PanelLeftOpen,
  MessageSquare,
  SettingsIcon,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useSidebarStore } from '@/hooks/useSidebarStore';
import { Button } from '@/components/ui/button';
import { useEnabledAiChatServices, useOpenAiChatWebview } from '@/hooks/useAiChatServices';

const SERVICE_ICONS: Record<string, string> = {
  deepseek: '🤖',
  chatgpt: '💬',
  claude: '🎭',
  gemini: '✨',
};

export function Sidebar() {
  const location = useLocation();
  const { t } = useTranslation();
  const { collapsed, toggle } = useSidebarStore();
  const { data: enabledChatServices = [] } = useEnabledAiChatServices();
  const openWebview = useOpenAiChatWebview();

  const navigation = [
    { name: t('nav.dashboard'), href: '/', icon: Home },
    { name: t('nav.accounts'), href: '/accounts', icon: UserCircle },
    { name: t('nav.tokens'), href: '/tokens', icon: Key },
    { name: t('nav.providers'), href: '/providers', icon: Server },
  ];

  // Helper for rendering links to avoid duplication between nav and settings
  const renderLink = (item: { name: string; href: string; icon: any }, isSettings = false) => {
    const isActive =
      location.pathname === item.href ||
      (!isSettings && item.href === '/accounts' &&
        (location.pathname.startsWith('/accounts') || location.pathname.startsWith('/account/')));
    const Icon = item.icon;

    const LinkContent = (
      <div
        className={cn(
          'relative group flex items-center transition-all duration-200 ease-out',
          collapsed
            ? 'justify-center w-10 h-10 rounded-xl'
            : 'w-full h-10 px-3 rounded-lg gap-3',
          isActive
            ? 'bg-primary/10 text-primary font-medium'
            : 'text-muted-foreground hover:bg-muted/50 hover:text-foreground'
        )}
      >
        <Icon className={cn("shrink-0 transition-transform duration-200", collapsed ? "h-5 w-5" : "h-4 w-4", isActive && "stroke-[2.5px]")} />
        {!collapsed && (
          <span className={cn(
            "text-sm whitespace-nowrap overflow-hidden transition-all duration-300",
            isActive ? "font-semibold" : ""
          )}>
            {item.name}
          </span>
        )}

        {/* Active Indicator for Collapsed Mode */}
        {collapsed && isActive && (
          <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-5 bg-primary rounded-r-full" />
        )}
      </div>
    );

    if (collapsed) {
      return (
        <Tooltip key={item.name} delayDuration={0}>
          <TooltipTrigger asChild>
            <Link to={item.href} className="w-full flex justify-center">
              {LinkContent}
            </Link>
          </TooltipTrigger>
          <TooltipContent side="right" className="ml-2 font-medium bg-popover text-popover-foreground border-border/50 shadow-macos">
            <p>{item.name}</p>
          </TooltipContent>
        </Tooltip>
      );
    }

    return (
      <Link key={item.name} to={item.href} className="w-full">
        {LinkContent}
      </Link>
    );
  };

  // Render an AI chat service button
  const renderChatService = (service: { id: string; name: string; url: string; icon: string | null }) => {
    const emoji = SERVICE_ICONS[service.icon || ''] || '🔗';

    const handleClick = () => {
      openWebview.mutate({
        id: service.id,
        name: service.name,
        url: service.url,
        icon: service.icon,
        is_builtin: true,
        is_enabled: true,
        sort_order: 0,
      });
    };

    const ServiceContent = (
      <button
        onClick={handleClick}
        className={cn(
          'relative group flex items-center transition-all duration-200 ease-out w-full',
          collapsed
            ? 'justify-center w-10 h-10 rounded-xl'
            : 'h-9 px-3 rounded-lg gap-3',
          'text-muted-foreground hover:bg-muted/50 hover:text-foreground'
        )}
      >
        <span className={cn("shrink-0", collapsed ? "text-lg" : "text-sm")}>{emoji}</span>
        {!collapsed && (
          <span className="text-sm whitespace-nowrap overflow-hidden text-ellipsis">
            {service.name}
          </span>
        )}
      </button>
    );

    if (collapsed) {
      return (
        <Tooltip key={service.id} delayDuration={0}>
          <TooltipTrigger asChild>
            <div className="w-full flex justify-center">
              {ServiceContent}
            </div>
          </TooltipTrigger>
          <TooltipContent side="right" className="ml-2 font-medium bg-popover text-popover-foreground border-border/50 shadow-macos">
            <p>{service.name}</p>
          </TooltipContent>
        </Tooltip>
      );
    }

    return (
      <div key={service.id} className="w-full">
        {ServiceContent}
      </div>
    );
  };

  return (
    <aside
      className={cn(
        "flex flex-col h-full items-center py-4 transition-all duration-300 select-none bg-sidebar/50 backdrop-blur-sm border-r border-border/40",
        collapsed ? "w-[72px]" : "w-40"
      )}
    >
      {/* Spacer for traffic lights (macOS) */}
      <div className="w-full h-6 shrink-0 mb-4" />

      {/* Navigation */}
      <nav
        className={cn(
          "w-full space-y-2 flex flex-col",
          collapsed ? "px-3 items-center" : "px-4 items-start"
        )}
      >
        {navigation.map((item) => renderLink(item))}
      </nav>

      {/* AI Chat Section */}
      {enabledChatServices.length > 0 && (
        <div className={cn(
          "w-full mt-4 flex flex-col gap-1",
          collapsed ? "px-3 items-center" : "px-4"
        )}>
          <div className="h-px w-full bg-border/40 mb-2" />

          {/* Section header */}
          {!collapsed && (
            <div className="flex items-center justify-between px-1 mb-1">
              <span className="text-xs font-semibold text-muted-foreground/60 uppercase tracking-wider">
                {t('nav.aiChat')}
              </span>
              <Link to="/ai-chat" className="text-muted-foreground/40 hover:text-muted-foreground transition-colors">
                <SettingsIcon className="h-3 w-3" />
              </Link>
            </div>
          )}
          {collapsed && (
            <Tooltip delayDuration={0}>
              <TooltipTrigger asChild>
                <Link to="/ai-chat" className="flex justify-center w-10 h-6 items-center text-muted-foreground/40 hover:text-muted-foreground">
                  <MessageSquare className="h-4 w-4" />
                </Link>
              </TooltipTrigger>
              <TooltipContent side="right" className="ml-2 font-medium bg-popover text-popover-foreground border-border/50 shadow-macos">
                <p>{t('nav.aiChat')}</p>
              </TooltipContent>
            </Tooltip>
          )}

          {/* Chat service items */}
          {enabledChatServices.map((service) => renderChatService(service))}
        </div>
      )}

      {/* Spacer */}
      <div className="flex-1" />

      {/* Footer / Settings / Toggle */}
      <div className={cn(
        "w-full mt-auto flex flex-col gap-2 shrink-0",
        collapsed ? "px-3 items-center" : "px-4"
      )}>
         {/* AI Chat settings link (show if no services enabled, as entry point) */}
         {enabledChatServices.length === 0 && renderLink({ name: t('nav.aiChat'), href: '/ai-chat', icon: MessageSquare })}

         {/* Settings Button */}
         {renderLink({ name: t('nav.settings'), href: '/settings', icon: Settings }, true)}

         <div className="h-px w-full bg-border/40 my-1" />

         <Button
            variant="ghost"
            size="icon"
            onClick={toggle}
            className={cn(
              "h-9 w-full flex items-center text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors",
              collapsed ? "justify-center rounded-xl" : "justify-start px-2 gap-3 rounded-lg"
            )}
         >
            {collapsed ? <PanelLeftOpen className="h-4 w-4" /> : <PanelLeftClose className="h-4 w-4" />}
            {!collapsed && <span className="text-sm">{t('common.collapse', { defaultValue: 'Collapse' })}</span>}
         </Button>
      </div>
    </aside>
  );
}
