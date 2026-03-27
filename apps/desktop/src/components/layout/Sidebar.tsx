import { Link, useLocation } from 'react-router-dom';
import {
  Bot,
  CalendarDays,
  Hammer,
  Key,
  Server,
  Settings,
  PanelLeftClose,
  PanelLeftOpen,
} from 'lucide-react';
import { cn } from '@/lib/utils';
// ... rest of imports

import { useTranslation } from 'react-i18next';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useSidebarStore } from '@/hooks/useSidebarStore';
import { Button } from '@/components/ui/button';
import { CodexMarkIcon } from '@/components/icons/CodexMarkIcon';

export function Sidebar() {
  const location = useLocation();
  const { t } = useTranslation();
  const { collapsed, toggle } = useSidebarStore();

  // Disable tooltips when AI Chat webview is active because native webview overlays DOM elements.
  const isOnAiChatPage = location.pathname.startsWith('/ai-chat');

  const navGroups = [
    {
      label: t('nav.aiToolbox', { defaultValue: 'AI工具箱' }),
      items: [
        { name: t('nav.providers'), href: '/providers', icon: Server },
        { name: t('nav.tokens'), href: '/tokens', icon: Key },
        { name: t('nav.aiChat'), href: '/ai-chat', icon: Bot },
        { name: t('nav.codex', { defaultValue: 'Codex' }), href: '/codex', icon: CodexMarkIcon },
      ],
    },
    {
      label: t('nav.devToolbox', { defaultValue: '开发工具箱' }),
      items: [
        {
          name: t('devToolbox.comingSoonTitle', '建设中'),
          href: '/dev-tools',
          icon: Hammer,
        },
      ],
    },
    {
      label: t('nav.dailyToolbox', { defaultValue: '日常工具箱' }),
      items: [{ name: t('nav.calendar'), href: '/calendar', icon: CalendarDays }],
    },
  ];

  // Helper for rendering links to avoid duplication between nav and settings
  const renderLink = (
    item: { name: string; href: string; icon: any },
    isSettings = false
  ) => {
    const normalizedHref = item.href.split('?')[0];
    const isActive =
      location.pathname === normalizedHref ||
      (!isSettings &&
        normalizedHref === '/providers' &&
        (location.pathname === '/' ||
          location.pathname === '/accounts' ||
          location.pathname.startsWith('/accounts/') ||
          location.pathname.startsWith('/account/') ||
          location.pathname.startsWith('/providers'))) ||
      (!isSettings && normalizedHref === '/ai-chat' && location.pathname.startsWith('/ai-chat')) ||
      (!isSettings && normalizedHref === '/calendar' && location.pathname.startsWith('/calendar')) ||
      (!isSettings && normalizedHref === '/dev-tools' && location.pathname.startsWith('/dev-tools'));
    const Icon = item.icon;

    const LinkContent = (
      <div
        className={cn(
          'relative group flex items-center transition-all duration-base ease-smooth',
          collapsed
            ? 'justify-center w-11 h-11 rounded-2xl'
            : 'w-full h-11 px-4 rounded-2xl gap-3',
          isActive
            ? 'bg-primary text-primary-foreground font-bold shadow-lg shadow-primary/30 scale-[1.02]'
            : 'text-muted-foreground hover:bg-primary/10 hover:text-primary hover:scale-[1.05]'
        )}
      >
        <Icon className={cn(
          "shrink-0 transition-all duration-base ease-smooth group-hover:animate-float",
          collapsed ? "h-6 w-6" : "h-5 w-5", 
          isActive && "stroke-[2.5px]"
        )} />
        {!collapsed && (
          <span className={cn(
            "text-sm tracking-tight whitespace-nowrap overflow-hidden transition-all duration-slow ease-smooth",
            isActive ? "font-bold" : "font-medium"
          )}>
            {item.name}
          </span>
        )}
      </div>
    );

    if (collapsed) {
      // On AI Chat page, disable tooltips (webview renders above DOM elements)
      if (isOnAiChatPage) {
        return (
          <Link key={item.name} to={item.href} className="w-full flex justify-center">
            {LinkContent}
          </Link>
        );
      }
      
      return (
        <Tooltip 
          key={item.name} 
          delayDuration={0}
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
        "flex flex-col h-full items-center py-4 transition-all duration-slow ease-smooth select-none bg-sidebar/50 backdrop-blur-sm border-r border-border/40",
        collapsed ? "w-[72px]" : "w-40"
      )}
    >
      {/* Spacer for traffic lights (macOS) */}
      <div className="w-full h-6 shrink-0 mb-4" />

      {/* Navigation */}
      <div className="w-full min-h-0 flex-1 overflow-y-auto scrollbar-hide">
        <nav
          className={cn(
            "w-full space-y-2 flex flex-col",
            collapsed ? "px-3 items-center" : "px-4 items-start"
          )}
        >
          {navGroups.map((group, gi) => (
            <div key={group.label} className="w-full">
              {gi > 0 && (
                <div
                  className={cn(
                    "my-2",
                    collapsed ? "h-px bg-border/40 w-8 mx-auto" : "h-px bg-border/40"
                  )}
                />
              )}
              {!collapsed && (
                <div className="text-[10px] uppercase tracking-widest text-muted-foreground/60 font-semibold px-1 mb-1.5">
                  {group.label}
                </div>
              )}
              <div className="w-full space-y-1">
                {group.items.map((item) => renderLink(item))}
              </div>
            </div>
          ))}
        </nav>
      </div>

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
              "h-9 w-full flex items-center text-muted-foreground hover:text-foreground hover:bg-muted/60 transition-all duration-base hover:scale-[1.02]",
              collapsed ? "justify-center rounded-xl" : "justify-start px-2 gap-3 rounded-xl"
            )}
         >
            {collapsed ? <PanelLeftOpen className="h-4 w-4" /> : <PanelLeftClose className="h-4 w-4" />}
            {!collapsed && <span className="text-sm">{t('common.collapse', { defaultValue: 'Collapse' })}</span>}
         </Button>
      </div>
    </aside>
  );
}
