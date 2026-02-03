import { ReactNode } from 'react';
import { Sidebar } from './Sidebar';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface MainLayoutProps {
  children: ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  const handleStartDrag = async (
    event: React.PointerEvent<HTMLDivElement> | React.MouseEvent<HTMLDivElement>
  ) => {
    if (event.button !== 0) return;
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;

    const interactiveSelector = [
      '[data-tauri-no-drag]',
      'button',
      'a',
      'input',
      'textarea',
      'select',
      '[role="button"]',
      '[role="combobox"]',
      '[role="listbox"]',
      '[role="menuitem"]',
      '[contenteditable="true"]',
      'label',
    ].join(',');

    const path = event.nativeEvent.composedPath?.() ?? [];
    if (path.length > 0) {
      for (const node of path) {
        if (node instanceof Element && node.matches(interactiveSelector)) return;
      }
    } else {
      const target = event.target;
      if (target instanceof Element && target.closest(interactiveSelector)) return;
    }

    event.preventDefault();
    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      // Surface failures during dev so we can see why dragging is blocked.
      console.warn('[drag] startDragging failed', error);
    }
  };

  return (
    <div
      className="flex h-screen overflow-hidden bg-sidebar text-foreground relative"
      onPointerDownCapture={handleStartDrag}
      onMouseDownCapture={handleStartDrag}
    >
      {/* Sidebar sits directly on the base background */}
      <Sidebar />

      {/* Main Content Area - Floating Canvas Style */}
      <main className="flex-1 min-w-0 p-2 pl-0 h-screen overflow-hidden relative">
        <div className="h-full w-full bg-background rounded-2xl shadow-sm border border-border/50 overflow-hidden flex flex-col relative">
          <div className="flex-1 overflow-hidden p-0 relative z-0">
            {children}
          </div>
        </div>
      </main>
    </div>
  );
}
