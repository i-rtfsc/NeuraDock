import { Link, useLocation } from 'react-router-dom';
import {
  Bot,
  CalendarDays,
  Key,
  Server,
  Settings,
  PanelLeftClose,
  PanelLeftOpen,
  MessageSquare,
  ChevronRight,
  Wrench,
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
import { useGatewayStatus } from '@/components/agent/AgentConfigGateway';
import { useEffect, useState } from 'react';
import type { ComponentType } from 'react';
import { commands } from '@/lib/tauri';

export function Sidebar() {
  const location = useLocation();
  const { t } = useTranslation();
  const { collapsed, toggle } = useSidebarStore();
  const [appVersion, setAppVersion] = useState<string>('');
  const { data: gatewayStatus } = useGatewayStatus();
  const gatewayRunning = gatewayStatus?.running ?? false;
  const devToolboxRouteActive =
    location.pathname.startsWith('/tokens') ||
    location.pathname.startsWith('/codex') ||
    location.pathname.startsWith('/calendar') ||
    location.pathname.startsWith('/dev-tools');
  const [devToolboxOpen, setDevToolboxOpen] = useState(devToolboxRouteActive);

  useEffect(() => {
    commands.getAppVersion()
      .then((fullVersion) => {
        const match = fullVersion.match(/^(.*) \((.*)\)$/);
        setAppVersion(match && match.length === 3 ? match[1] : fullVersion);
      })
      .catch(() => setAppVersion(''));
  }, []);

  useEffect(() => {
    if (devToolboxRouteActive) setDevToolboxOpen(true);
  }, [devToolboxRouteActive, location.pathname]);

  // Disable tooltips when AI Chat webview is active because native webview overlays DOM elements.
  const isOnAiChatPage = location.pathname.startsWith('/ai-chat');

  const navGroups = [
    {
      label: t('nav.aiToolbox', { defaultValue: 'AI工具箱' }),
      items: [
        { name: t('nav.providers'), href: '/providers', icon: Server },
        { name: t('nav.agentConfig', 'Agent 配置'), href: '/agent-config', icon: Bot, gatewayStatus: true },
        { name: t('nav.aiChat'), href: '/ai-chat', icon: MessageSquare },
      ],
    },
    {
      label: t('nav.devToolbox', { defaultValue: '开发工具箱' }),
      collapsible: true,
      items: [
        { name: t('nav.tokens'), href: '/tokens', icon: Key },
        { name: t('nav.codex', { defaultValue: 'Codex' }), href: '/codex', icon: CodexMarkIcon },
        { name: t('nav.calendar'), href: '/calendar', icon: CalendarDays },
      ],
    },
  ];

  // Helper for rendering links to avoid duplication between nav and settings
  const renderLink = (
    item: { name: string; href: string; icon: ComponentType<{ className?: string }>; gatewayStatus?: boolean },
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
      (!isSettings && normalizedHref === '/agent-config' && location.pathname.startsWith('/agent-config')) ||
      (!isSettings && normalizedHref === '/calendar' && location.pathname.startsWith('/calendar')) ||
      (!isSettings && normalizedHref === '/dev-tools' && location.pathname.startsWith('/dev-tools'));
    const Icon = item.icon;
    const showGatewayDot = !!item.gatewayStatus;

    const LinkContent = (
      <div
        className={cn(
          'relative group flex items-center transition-all duration-base ease-smooth',
          collapsed
            ? 'h-9 w-9 justify-center rounded-xl'
            : 'h-9 w-full gap-2.5 rounded-xl px-3',
          isActive
            ? 'bg-primary text-primary-foreground font-bold shadow-md shadow-primary/20'
            : 'text-muted-foreground hover:bg-primary/10 hover:text-primary'
        )}
      >
        <span className="relative shrink-0 flex items-center">
          <Icon className={cn(
            "h-[18px] w-[18px] shrink-0 transition-colors duration-base ease-smooth",
            isActive && "stroke-[2.5px]"
          )} />
          {showGatewayDot && (
            <span
              className={cn(
                'absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full ring-2 ring-background',
                gatewayRunning ? 'bg-success' : 'bg-muted-foreground/40'
              )}
            />
          )}
        </span>
        {!collapsed && (
          <span className={cn(
            "flex-1 overflow-hidden whitespace-nowrap text-[13px] tracking-tight transition-all duration-slow ease-smooth",
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
        "flex h-full shrink-0 flex-col items-center border-r border-border/40 bg-sidebar/50 py-3 backdrop-blur-sm transition-all duration-slow ease-smooth select-none",
        collapsed ? "w-16" : "w-40"
      )}
    >
      {/* Spacer for traffic lights (macOS) */}
      <div className="mb-3 h-6 w-full shrink-0" />

      {/* Navigation */}
      <div className="w-full min-h-0 flex-1 overflow-y-auto scrollbar-hide">
        <nav
          className={cn(
            "flex w-full flex-col space-y-1.5",
            collapsed ? "px-2.5 items-center" : "px-3 items-start"
          )}
        >
          {navGroups.map((group) => {
            const isCollapsibleGroup = 'collapsible' in group && group.collapsible;
            const groupOpen = isCollapsibleGroup ? devToolboxOpen : true;
            const groupHasActiveItem = group.items.some((item) => {
              const href = item.href.split('?')[0];
              return location.pathname === href || location.pathname.startsWith(`${href}/`);
            });
            const collapsedGroupToggle = isCollapsibleGroup ? (
              <button
                type="button"
                onClick={() => setDevToolboxOpen((value) => !value)}
                className={cn(
                  'relative mx-auto flex h-9 w-9 items-center justify-center rounded-xl transition-colors duration-base',
                  groupHasActiveItem
                    ? 'bg-primary/10 text-primary'
                    : groupOpen
                      ? 'bg-muted/60 text-foreground'
                      : 'text-muted-foreground hover:bg-primary/10 hover:text-primary'
                )}
                aria-label={group.label}
                aria-expanded={groupOpen}
              >
                <Wrench className="h-[18px] w-[18px]" />
                <ChevronRight
                  className={cn(
                    'absolute bottom-0.5 right-0.5 h-3 w-3 rounded-full bg-sidebar transition-transform duration-base',
                    groupOpen && 'rotate-90'
                  )}
                />
              </button>
            ) : null;
            return (
              <div key={group.label} className="w-full">
                {collapsed && isCollapsibleGroup &&
                  (isOnAiChatPage ? (
                    collapsedGroupToggle
                  ) : (
                    <Tooltip delayDuration={0}>
                      <TooltipTrigger asChild>{collapsedGroupToggle}</TooltipTrigger>
                      <TooltipContent
                        side="right"
                        className="ml-2 border-border/50 bg-popover font-medium text-popover-foreground shadow-macos"
                      >
                        <p>{group.label}</p>
                      </TooltipContent>
                    </Tooltip>
                  ))}
                {!collapsed && (
                  isCollapsibleGroup ? (
                    <button
                      type="button"
                      onClick={() => setDevToolboxOpen((v) => !v)}
                      className="w-full flex items-center gap-1 text-[10px] tracking-widest text-muted-foreground/60 font-semibold px-1 mb-1.5 hover:text-muted-foreground transition-colors"
                    >
                      <ChevronRight
                        className={cn(
                          'h-3 w-3 transition-transform duration-base',
                          groupOpen && 'rotate-90'
                        )}
                      />
                      {group.label}
                    </button>
                  ) : (
                    <div className="text-[10px] tracking-widest text-muted-foreground/60 font-semibold px-1 mb-1.5">
                      {group.label}
                    </div>
                  )
                )}
                {groupOpen && (
                  <div className={cn('w-full space-y-0.5', collapsed && isCollapsibleGroup && 'mt-1')}>
                    {group.items.map((item) => renderLink(item))}
                  </div>
                )}
              </div>
            );
          })}
        </nav>
      </div>

      {/* Footer / Settings / Toggle */}
      <div className={cn(
        "mt-auto flex w-full shrink-0 flex-col gap-1.5",
        collapsed ? "px-2.5 items-center" : "px-3"
      )}>
         {/* Settings Button */}
         {renderLink({ name: t('nav.settings'), href: '/settings', icon: Settings }, true)}

         <div className="h-px w-full bg-border/40 my-1" />

         <Button
            variant="ghost"
            size="icon"
            onClick={toggle}
            className={cn(
              "flex h-9 w-full items-center text-muted-foreground transition-colors duration-base hover:bg-muted/60 hover:text-foreground",
              collapsed ? "justify-center rounded-xl" : "justify-start gap-2.5 rounded-xl px-3"
            )}
         >
            {collapsed ? <PanelLeftOpen className="h-4 w-4" /> : <PanelLeftClose className="h-4 w-4" />}
            {!collapsed && <span className="text-[13px]">{t('common.collapse', { defaultValue: 'Collapse' })}</span>}
         </Button>

         {!!appVersion && (
           <div
             className={cn(
               'pt-1 pb-0.5 font-mono text-muted-foreground/70',
               collapsed
                 ? 'w-[52px] text-center text-[9px] leading-tight truncate'
                 : 'w-full text-center text-[11px]'
             )}
             title={`v${appVersion}`}
           >
             v{appVersion}
           </div>
         )}
      </div>
    </aside>
  );
}
