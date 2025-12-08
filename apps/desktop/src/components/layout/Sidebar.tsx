import { Link, useLocation } from 'react-router-dom';
import {
  Home,
  UserCircle,
  Flame,
  Key,
  Settings2
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useState, useEffect } from 'react';

export function Sidebar() {
  const location = useLocation();
  const { t } = useTranslation();
  const [collapsed, setCollapsed] = useState(false);

  useEffect(() => {
    const checkState = () => {
      const stored = localStorage.getItem('sidebarCollapsed');
      setCollapsed(stored === 'true');
    };

    // Initial check
    checkState();

    // Listen for toggle events
    window.addEventListener('sidebarToggle', checkState);
    return () => window.removeEventListener('sidebarToggle', checkState);
  }, []);
  
  const navigation = [
    { name: t('nav.dashboard'), href: '/', icon: Home },
    { name: t('nav.accounts'), href: '/accounts', icon: UserCircle },
    { name: t('nav.streaks'), href: '/streaks', icon: Flame },
    { name: t('nav.tokens'), href: '/tokens', icon: Key },
    { name: t('nav.settings'), href: '/settings', icon: Settings2 },
  ];

  return (
    <aside 
      className={cn(
        "flex flex-col h-full items-center pt-10 pb-4 transition-all duration-300 select-none bg-sidebar/50 backdrop-blur-sm",
        collapsed ? "w-[72px]" : "w-32"
      )}
      data-tauri-drag-region
    >
      {/* App Logo or Drag Region */}
      <div className="h-4 w-full mb-6 shrink-0" data-tauri-drag-region />

      {/* Navigation */}
      <nav className={cn(
        "flex-1 w-full space-y-2 flex flex-col",
        collapsed ? "px-3 items-center" : "px-4 items-start"
      )}>
        {navigation.map((item) => {
          const isActive = location.pathname === item.href;
          const Icon = item.icon;

          const LinkContent = (
             <div
               className={cn(
                 'relative group flex items-center transition-all duration-200 ease-out',
                 collapsed 
                   ? 'justify-center w-10 h-10 rounded-xl' 
                   : 'w-full h-11 px-3 rounded-xl gap-3',
                 isActive
                   ? 'bg-primary text-primary-foreground shadow-md'
                   : 'text-muted-foreground hover:bg-sidebar-accent/10 hover:text-foreground'
               )}
             >
               <Icon className={cn("shrink-0", collapsed ? "h-5 w-5" : "h-5 w-5", isActive && "stroke-[2.5px]")} />
               {!collapsed && (
                 <span className={cn(
                   "font-medium text-sm whitespace-nowrap overflow-hidden transition-all duration-300",
                   isActive ? "font-semibold" : ""
                 )}>
                   {item.name}
                 </span>
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
        })}
      </nav>
      
      {/* Footer Toggle (Optional, usually handled in settings, but nice to have here too) */}
      {!collapsed && (
         <div className="w-full px-4 mt-auto">
            <div className="text-xs text-muted-foreground/50 text-center py-2">
              v0.1.0
            </div>
         </div>
      )}
    </aside>
  );
}
