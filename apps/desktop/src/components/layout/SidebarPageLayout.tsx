import { ReactNode } from 'react';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';

interface SidebarPageLayoutProps {
  sidebar: ReactNode;
  children: ReactNode;
  className?: string;
  sidebarWidth?: string;
}

export function SidebarPageLayout({ 
  sidebar, 
  children, 
  className,
  sidebarWidth = "w-60" 
}: SidebarPageLayoutProps) {
  return (
    <div className={cn("flex flex-row gap-6 h-full overflow-hidden pl-1 pt-1", className)}>
      {/* Left Sidebar */}
      <div className={cn("flex flex-col shrink-0 gap-4", sidebarWidth)}>
        {sidebar}
      </div>

      {/* Right Content */}
      <div className="flex-1 flex flex-col overflow-hidden min-w-0">
        <ScrollArea className="flex-1 -mr-4 pr-4">
          <div className="pl-1 pt-1 pr-1 pb-6">
            {children}
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
