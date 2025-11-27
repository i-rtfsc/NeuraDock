import { Link, useLocation } from 'react-router-dom';
import { 
  Home,
  UserCircle, 
  Settings2
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import { useState, useEffect } from 'react';

export function Sidebar() {
  const location = useLocation();
  const { t } = useTranslation();
  
  // Load collapsed state from localStorage
  const [collapsed, setCollapsed] = useState(() => {
    const stored = localStorage.getItem('sidebarCollapsed');
    return stored === 'true';
  });

  // Listen for changes from settings page
  useEffect(() => {
    const handleStorageChange = () => {
      const stored = localStorage.getItem('sidebarCollapsed');
      setCollapsed(stored === 'true');
    };

    window.addEventListener('storage', handleStorageChange);
    // Also listen for custom event from same window
    window.addEventListener('sidebarToggle', handleStorageChange);

    return () => {
      window.removeEventListener('storage', handleStorageChange);
      window.removeEventListener('sidebarToggle', handleStorageChange);
    };
  }, []);

  const navigation = [
    { name: t('nav.dashboard'), href: '/', icon: Home },
    { name: t('nav.accounts'), href: '/accounts', icon: UserCircle },
    { name: t('nav.settings'), href: '/settings', icon: Settings2 },
  ];

  return (
    <div 
      className={cn(
        'flex flex-col h-full bg-background transition-all duration-300 select-none w-[80px]'
      )}
    >
      {/* Navigation - with proper 12px margins on both sides */}
      <nav className="flex-1 py-12">
        {navigation.map((item) => {
          const isActive = location.pathname === item.href;
          const Icon = item.icon;

          return (
            <Link key={item.name} to={item.href}>
              <div
                className={cn(
                  'flex items-center justify-center py-3 mb-1 cursor-pointer transition-all rounded-lg mx-3',
                  isActive
                    ? 'bg-primary text-primary-foreground shadow-sm'
                    : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                  collapsed ? 'px-0' : 'px-2 flex-col gap-1'
                )}
                title={collapsed ? item.name : undefined}
              >
                <Icon className="h-5 w-5" />
                {!collapsed && <span className="text-xs font-medium truncate text-center">{item.name}</span>}
              </div>
            </Link>
          );
        })}
      </nav>
    </div>
  );
}
