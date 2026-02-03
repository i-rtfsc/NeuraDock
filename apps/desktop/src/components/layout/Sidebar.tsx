import { Link, useLocation } from 'react-router-dom';
import {
  Home,
  UserCircle,
  Key,
  Server,
  Settings,
  PanelLeftClose,
  PanelLeftOpen,
  Bot,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useSidebarStore } from '@/hooks/useSidebarStore';
import { useAiChatStore } from '@/hooks/useAiChatStore';
import { Button } from '@/components/ui/button';

export function Sidebar() {
  const location = useLocation();
  const { t } = useTranslation();
  const { collapsed, toggle } = useSidebarStore();
  const setWebviewHidden = useAiChatStore((state) => state.setWebviewHidden);
  
  // Check if we're on the AI Chat page
  const isOnAiChatPage = location.pathname.startsWith('/ai-chat');

  const navigation = [
    { name: t('nav.dashboard'), href: '/', icon: Home },
    { name: t('nav.accounts'), href: '/accounts', icon: UserCircle },
    { name: t('nav.tokens'), href: '/tokens', icon: Key },
    { name: t('nav.providers'), href: '/providers', icon: Server },
    { name: t('nav.aiChat'), href: '/ai-chat', icon: Bot },
  ];

  // Helper for rendering links to avoid duplication between nav and settings
  const renderLink = (item: { name: string; href: string; icon: any }, isSettings = false) => {
    const isActive =
      location.pathname === item.href ||
      (!isSettings && item.href === '/accounts' &&
        (location.pathname.startsWith('/accounts') || location.pathname.startsWith('/account/'))) ||
      (!isSettings && item.href === '/ai-chat' && location.pathname.startsWith('/ai-chat'));
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
        <Tooltip 
          key={item.name} 
          delayDuration={0}
          onOpenChange={(open) => {
            // Hide webview when tooltip opens (only matters on AI Chat page)
            if (isOnAiChatPage) {
              setWebviewHidden(open);
            }
          }}
        >
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

      {/* Spacer */}
      <div className="flex-1" />

      {/* Footer / Settings / Toggle */}
      <div className={cn(
        "w-full mt-auto flex flex-col gap-2 shrink-0",
        collapsed ? "px-3 items-center" : "px-4"
      )}>
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
