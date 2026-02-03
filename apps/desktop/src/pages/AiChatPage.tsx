import { useState, useEffect, useRef, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Settings, Plus, X, RefreshCw, ExternalLink, AppWindow } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { LoadingState } from '@/components/ui/loading';
import {
  useEnabledAiChatServices,
  AiChatServiceDto,
} from '@/hooks/useAiChatServices';
import { useAiChatStore } from '@/hooks/useAiChatStore';
import { invoke } from '@tauri-apps/api/core';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';

interface TabProps {
  service: AiChatServiceDto;
  isActive: boolean;
  onClick: () => void;
  onClose: () => void;
}

function Tab({ service, isActive, onClick, onClose }: TabProps) {
  return (
    <div
      className={cn(
        'group flex items-center gap-2 px-3 py-2 rounded-t-lg cursor-pointer transition-all border-b-2 min-w-[120px] max-w-[180px]',
        isActive
          ? 'bg-background border-primary text-foreground'
          : 'bg-muted/30 border-transparent text-muted-foreground hover:bg-muted/50 hover:text-foreground'
      )}
      onClick={onClick}
    >
      <span className="text-sm font-medium truncate flex-1">{service.name}</span>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onClose();
        }}
        className="opacity-0 group-hover:opacity-100 hover:bg-muted rounded p-0.5 transition-opacity"
      >
        <X className="h-3 w-3" />
      </button>
    </div>
  );
}

export function AiChatPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { data: enabledServices = [], isLoading } = useEnabledAiChatServices();
  const { openTabs, activeTabId, openTab, closeTab, setActiveTab } = useAiChatStore();

  const containerRef = useRef<HTMLDivElement>(null);
  const [webviewReady, setWebviewReady] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);

  // Get the active service
  const activeService = openTabs.find((tab) => tab.serviceId === activeTabId);

  // Create or update the embedded webview when active tab changes
  useEffect(() => {
    // Hide webview when dropdown is open (native webview overlays DOM elements)
    if (isDropdownOpen) {
      invoke('hide_embedded_ai_chat').catch(console.error);
      return;
    }

    if (!activeService || !containerRef.current) {
      // Hide webview when no active service
      invoke('hide_embedded_ai_chat').catch(console.error);
      setWebviewReady(false);
      return;
    }

    const updateWebview = async () => {
      try {
        const rect = containerRef.current?.getBoundingClientRect();
        if (!rect) return;

        await invoke('show_embedded_ai_chat', {
          serviceId: activeService.serviceId,
          url: activeService.url,
          x: Math.round(rect.left),
          y: Math.round(rect.top),
          width: Math.round(rect.width),
          height: Math.round(rect.height),
        });
        setWebviewReady(true);
      } catch (error) {
        console.error('Failed to show embedded webview:', error);
      }
    };

    updateWebview();

    // Update position on resize
    const handleResize = () => {
      updateWebview();
    };
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
    };
  }, [activeService, isDropdownOpen]);

  // Hide webview when component unmounts or navigates away
  useEffect(() => {
    return () => {
      invoke('hide_embedded_ai_chat').catch(console.error);
    };
  }, []);

  const handleOpenService = useCallback((service: AiChatServiceDto) => {
    openTab({
      serviceId: service.id,
      name: service.name,
      url: service.url,
      icon: service.icon ?? undefined,
    });
  }, [openTab]);

  const handleCloseTab = useCallback(async (serviceId: string) => {
    closeTab(serviceId);
    // If we're closing the active tab, the store will switch to another tab
    // The useEffect will handle hiding/showing the webview
  }, [closeTab]);

  const handleRefresh = async () => {
    if (!activeService) return;
    setIsRefreshing(true);
    try {
      await invoke('refresh_embedded_ai_chat');
    } catch (error) {
      console.error('Failed to refresh:', error);
    } finally {
      setIsRefreshing(false);
    }
  };

  const handleOpenInBrowser = async () => {
    if (!activeService) return;
    try {
      await invoke('open_ai_chat_in_browser', { url: activeService.url });
    } catch (error) {
      console.error('Failed to open in browser:', error);
    }
  };

  const handleOpenInWindow = async () => {
    if (!activeService) return;
    try {
      await invoke('open_ai_chat_webview', {
        serviceId: activeService.serviceId,
        serviceName: activeService.name,
        url: activeService.url,
      });
    } catch (error) {
      console.error('Failed to open in window:', error);
    }
  };

  if (isLoading) {
    return <LoadingState className="h-full" />;
  }

  // No enabled services - show empty state
  if (enabledServices.length === 0) {
    return (
      <div className="h-full flex flex-col items-center justify-center bg-muted/10 p-8">
        <div className="text-6xl mb-4">💬</div>
        <h2 className="text-xl font-semibold text-foreground mb-2">{t('aiChat.noServices')}</h2>
        <p className="text-muted-foreground text-center mb-6 max-w-md">
          {t('aiChat.noServicesDescription')}
        </p>
        <Button onClick={() => navigate('/ai-chat/settings')}>
          <Settings className="h-4 w-4 mr-2" />
          {t('aiChat.manageServices')}
        </Button>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-muted/10">
      {/* Header with tabs */}
      <div className="flex items-center border-b border-border/40 bg-background/50 backdrop-blur-sm shrink-0">
        {/* Tabs */}
        <div className="flex-1 flex items-end gap-1 px-2 pt-2 overflow-x-auto">
          {openTabs.map((tab) => (
            <Tab
              key={tab.serviceId}
              service={{
                id: tab.serviceId,
                name: tab.name,
                url: tab.url,
                icon: tab.icon || null,
                is_builtin: false,
                is_enabled: true,
                sort_order: 0,
              }}
              isActive={tab.serviceId === activeTabId}
              onClick={() => setActiveTab(tab.serviceId)}
              onClose={() => handleCloseTab(tab.serviceId)}
            />
          ))}

          {/* Add tab dropdown - use click instead of hover */}
          <DropdownMenu onOpenChange={setIsDropdownOpen}>
            <DropdownMenuTrigger asChild>
              <button className="flex items-center justify-center w-8 h-8 rounded hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors mx-1 my-2">
                <Plus className="h-4 w-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="min-w-[160px]">
              {enabledServices.map((service) => (
                <DropdownMenuItem
                  key={service.id}
                  onClick={() => handleOpenService(service)}
                  className="flex items-center gap-2 cursor-pointer"
                >
                  <span className="truncate">{service.name}</span>
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        {/* Actions */}
        <div className="flex items-center gap-1 px-2 py-2 shrink-0">
          {activeService && (
            <>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={handleRefresh}
                    disabled={isRefreshing}
                    className="h-8 w-8"
                  >
                    <RefreshCw className={cn("h-4 w-4", isRefreshing && "animate-spin")} />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{t('common.refresh')}</TooltipContent>
              </Tooltip>
              
              {/* Open options dropdown */}
              <DropdownMenu onOpenChange={setIsDropdownOpen}>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="icon" className="h-8 w-8">
                    <ExternalLink className="h-4 w-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onClick={handleOpenInWindow}>
                    <AppWindow className="h-4 w-4 mr-2" />
                    {t('aiChat.openInWindow')}
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={handleOpenInBrowser}>
                    <ExternalLink className="h-4 w-4 mr-2" />
                    {t('aiChat.openInBrowser')}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </>
          )}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => navigate('/ai-chat/settings')}
                className="h-8 w-8"
              >
                <Settings className="h-4 w-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t('aiChat.settings')}</TooltipContent>
          </Tooltip>
        </div>
      </div>

      {/* WebView container */}
      <div ref={containerRef} className="flex-1 relative">
        {!activeService && openTabs.length === 0 && (
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <p className="text-muted-foreground mb-4">{t('aiChat.selectService')}</p>
            <div className="flex flex-wrap gap-2 justify-center">
              {enabledServices.map((service) => (
                <Button
                  key={service.id}
                  variant="outline"
                  onClick={() => handleOpenService(service)}
                  className="gap-2"
                >
                  {service.name}
                </Button>
              ))}
            </div>
          </div>
        )}

        {activeService && !webviewReady && (
          <div className="absolute inset-0 flex items-center justify-center">
            <LoadingState />
          </div>
        )}
      </div>
    </div>
  );
}
