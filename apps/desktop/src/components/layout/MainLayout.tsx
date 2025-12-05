import { ReactNode } from 'react';
import { Sidebar } from './Sidebar';

interface MainLayoutProps {
  children: ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <main className="flex-1 overflow-auto p-3 pl-0 bg-background" data-tauri-drag-region>
        <div className="max-w-7xl mx-auto h-full" data-tauri-drag-region>
          <div className="bg-card rounded-3xl shadow-sm border h-full p-6 overflow-auto scrollbar-hide">
            {children}
          </div>
        </div>
      </main>
    </div>
  );
}
