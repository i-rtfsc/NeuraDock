import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Server } from 'lucide-react';

import { cn } from '@/lib/utils';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { GatewayPanelDialog, useGatewayStatus } from '@/components/gateway/GatewayPanelDialog';

interface SidebarGatewaySwitchProps {
  collapsed: boolean;
  tooltipsDisabled?: boolean;
}

/// Always-visible gateway status + entry in the sidebar footer, so the running
/// state is obvious from any page. Clicking opens the full gateway panel.
export function SidebarGatewaySwitch({ collapsed, tooltipsDisabled }: SidebarGatewaySwitchProps) {
  const { t } = useTranslation();
  const { data: status } = useGatewayStatus();
  const [panelOpen, setPanelOpen] = useState(false);

  const running = status?.running ?? false;
  const label = t('gateway.sidebarLabel', '本地网关');
  const stateText = running
    ? t('gateway.running', '运行中')
    : t('gateway.stoppedBadge', '已停止');

  const dot = (
    <span
      className={cn(
        'inline-block h-2 w-2 rounded-full shrink-0 transition-colors',
        running ? 'bg-success shadow-[0_0_6px_var(--success)]' : 'bg-muted-foreground/40'
      )}
    />
  );

  const trigger = (
    <button
      type="button"
      onClick={() => setPanelOpen(true)}
      className={cn(
        'h-9 w-full flex items-center rounded-xl text-muted-foreground hover:text-foreground hover:bg-muted/60 transition-all duration-base',
        collapsed ? 'justify-center' : 'justify-start px-2 gap-3'
      )}
    >
      {collapsed ? (
        <span className="relative flex items-center justify-center">
          <Server className="h-4 w-4" />
          <span className="absolute -top-1 -right-1">{dot}</span>
        </span>
      ) : (
        <>
          <Server className="h-4 w-4 shrink-0" />
          <span className="flex-1 text-left text-sm truncate">{label}</span>
          {dot}
        </>
      )}
    </button>
  );

  return (
    <>
      {collapsed && !tooltipsDisabled ? (
        <Tooltip>
          <TooltipTrigger asChild>{trigger}</TooltipTrigger>
          <TooltipContent side="right">
            {label} · {stateText}
          </TooltipContent>
        </Tooltip>
      ) : (
        trigger
      )}
      <GatewayPanelDialog open={panelOpen} onOpenChange={setPanelOpen} />
    </>
  );
}
