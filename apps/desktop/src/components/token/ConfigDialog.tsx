import { useTranslation } from 'react-i18next';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Badge } from '@/components/ui/badge';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import {
  AlertTriangle,
  Terminal,
  Code2,
  Sparkles,
  Server,
  ChevronRight,
  Zap,
  HardDrive,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { TokenDto, AccountDto } from '@/types/token';
import { useConfigState, type AITool } from '@/hooks/useConfigState';
import { ModelSelector } from './ModelSelector';
import { NodeSelector } from './NodeSelector';
import { CommandGenerator } from './CommandGenerator';

interface ConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  token: TokenDto | null;
  account: AccountDto | null;
}

const TOOL_OPTIONS = [
  {
    id: 'claude' as AITool,
    nameKey: 'token.configDialog.tools.claude',
    defaultName: 'Claude Code',
    descKey: 'token.configDialog.tools.claudeDesc',
    defaultDesc: 'Anthropic CLI',
    icon: Terminal,
    disabled: false,
  },
  {
    id: 'codex' as AITool,
    nameKey: 'token.configDialog.tools.codex',
    defaultName: 'Codex',
    descKey: 'token.configDialog.tools.codexDesc',
    defaultDesc: 'OpenAI Compatible',
    icon: Code2,
    disabled: false,
  },
  {
    id: 'gemini' as AITool,
    nameKey: 'token.configDialog.tools.gemini',
    defaultName: 'Gemini',
    descKey: 'token.configDialog.tools.geminiDesc',
    defaultDesc: 'Google AI',
    icon: Sparkles,
    disabled: true,
  },
];

export function ConfigDialog({
  open,
  onOpenChange,
  token,
  account,
}: ConfigDialogProps) {
  const { t } = useTranslation();

  const {
    selectedTool,
    setSelectedTool,
    selectedNode,
    setSelectedNode,
    nodes,
    selectedModel,
    setSelectedModel,
    filteredModels,
    isModelListLoading,
    tempCommands,
    setTempCommands,
    copied,
    setCopied,
    isSingleLine,
    setIsSingleLine,
    displayCommands,
    compatibilityWarning,
    isCompatible,
    modelLimitsEnabled,
  } = useConfigState({ open, token, account });

  if (!token || !account) return null;

  const currentTool = TOOL_OPTIONS.find((t) => t.id === selectedTool);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl p-0 gap-0 overflow-hidden h-[600px] flex flex-col bg-zinc-50/50 dark:bg-zinc-950/50 backdrop-blur-xl">
        <div className="flex h-full">
          {/* Left Sidebar - Tool Selection */}
          <div className="w-64 bg-background/80 border-r flex flex-col shrink-0 backdrop-blur-sm">
            <div className="p-5 border-b space-y-3">
              <div>
                <h2 className="font-semibold text-xs text-muted-foreground uppercase tracking-wider mb-1">
                  {t('token.selectTool', 'Integrations')}
                </h2>
                <div className="flex items-center gap-2 text-xs">
                  <span
                    className="font-medium text-primary truncate max-w-[140px]"
                    title={token.name}
                  >
                    {token.name}
                  </span>
                  {modelLimitsEnabled && (
                    <Badge
                      variant="secondary"
                      className="h-4 px-1 text-[9px] rounded-sm"
                    >
                      Limits
                    </Badge>
                  )}
                </div>
              </div>
              <p className="text-[10px] text-muted-foreground/70">
                {t(
                  'token.configDialog.toolDescription',
                  'Select the tool you want to configure'
                )}
              </p>
            </div>
            <ScrollArea className="flex-1">
              <div className="p-3 space-y-1">
                {TOOL_OPTIONS.map((tool) => {
                  const Icon = tool.icon;
                  const isSelected = selectedTool === tool.id;
                  return (
                    <button
                      key={tool.id}
                      onClick={() => !tool.disabled && setSelectedTool(tool.id)}
                      disabled={tool.disabled}
                      className={cn(
                        'w-full flex items-center gap-3 px-3 py-3 rounded-xl text-left transition-all duration-base ease-smooth',
                        isSelected
                          ? 'bg-primary text-primary-foreground shadow-md shadow-primary/20'
                          : 'text-muted-foreground hover:bg-muted/80 hover:text-foreground',
                        tool.disabled && 'opacity-50 cursor-not-allowed'
                      )}
                    >
                      <div
                        className={cn(
                          'p-2 rounded-lg shrink-0 transition-colors',
                          isSelected
                            ? 'bg-primary-foreground/20 text-primary-foreground'
                            : 'bg-muted text-muted-foreground'
                        )}
                      >
                        <Icon className="h-4 w-4" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="font-medium text-sm">
                          {t(tool.nameKey, tool.defaultName)}
                        </div>
                        <div
                          className={cn(
                            'text-xs truncate opacity-80',
                            isSelected
                              ? 'text-primary-foreground/80'
                              : 'text-muted-foreground'
                          )}
                        >
                          {t(tool.descKey, tool.defaultDesc)}
                        </div>
                      </div>
                      {isSelected && <ChevronRight className="h-4 w-4 opacity-50" />}
                    </button>
                  );
                })}
              </div>
            </ScrollArea>
            <div className="p-4 border-t bg-muted/10">
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <div className="h-2 w-2 rounded-full bg-green-500 animate-pulse" />
                {t('token.configDialog.configuringLabel', 'Configuring')}{' '}
                <span className="font-medium text-foreground truncate max-w-[120px]">
                  {token.name}
                </span>
              </div>
            </div>
          </div>

          {/* Right Content - Configuration */}
          <div className="flex-1 flex flex-col min-w-0 bg-background/50">
            <DialogHeader className="px-8 py-6 border-b shrink-0 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <DialogTitle className="flex items-center gap-3 text-2xl font-bold tracking-tight">
                    {currentTool && t(currentTool.nameKey, currentTool.defaultName)}
                    {selectedTool === 'gemini' && (
                      <Badge variant="secondary" className="text-xs font-normal">
                        {t('token.configDialog.comingSoon', 'Coming Soon')}
                      </Badge>
                    )}
                  </DialogTitle>
                  <p className="text-sm text-muted-foreground">
                    {t(
                      'token.configDialog.subtitle',
                      'Configure connection settings and credentials.'
                    )}
                  </p>
                </div>
                <div className="flex flex-col items-end gap-1">
                  <Badge
                    variant="outline"
                    className="gap-1.5 py-1 px-2 text-xs font-medium bg-background/50"
                  >
                    <Server className="h-3 w-3 text-primary" />
                    <span className="opacity-70">
                      {t('token.configDialog.provider', 'Provider')}:
                    </span>
                    {token.provider_name}
                  </Badge>
                </div>
              </div>
            </DialogHeader>

            <ScrollArea className="flex-1">
              <div className="p-8 space-y-6">
                {selectedTool === 'gemini' ? (
                  <div className="flex flex-col items-center justify-center h-64 text-center text-muted-foreground space-y-4">
                    <div className="p-6 rounded-full bg-muted/50 ring-1 ring-border">
                      <Sparkles className="h-10 w-10 opacity-50" />
                    </div>
                    <div className="space-y-1">
                      <h3 className="font-medium text-foreground">
                        {t('token.configDialog.comingSoon', 'Coming Soon')}
                      </h3>
                      <p className="text-sm">
                        {t(
                          'token.geminiSupportComingSoon',
                          'Support for Gemini is currently under development.'
                        )}
                      </p>
                    </div>
                  </div>
                ) : (
                  <>
                    {/* Compatibility Alert */}
                    {compatibilityWarning && (
                      <Alert
                        variant={isCompatible ? 'default' : 'destructive'}
                        className="animate-in fade-in slide-in-from-top-2"
                      >
                        <AlertTriangle className="h-4 w-4" />
                        <AlertDescription>{compatibilityWarning}</AlertDescription>
                      </Alert>
                    )}

                    {/* Connection Settings */}
                    <div className="space-y-4">
                      <h3 className="text-xs font-semibold text-muted-foreground tracking-wider flex items-center gap-2">
                        <Zap className="h-3.5 w-3.5" />
                        {t('token.configDialog.connectionSetup', 'Connection Setup')}
                      </h3>

                      <div className="grid gap-6 md:grid-cols-2">
                        <NodeSelector
                          providerId={account.provider_id}
                          selectedNode={selectedNode}
                          onNodeChange={setSelectedNode}
                          nodes={nodes}
                          onAfterNavigate={() => onOpenChange(false)}
                        />

                        <ModelSelector
                          selectedModel={selectedModel}
                          onModelChange={setSelectedModel}
                          filteredModels={filteredModels}
                          isLoading={isModelListLoading}
                        />
                      </div>
                    </div>

                    <Separator />

                    {/* Configuration Mode Tabs */}
                    <div className="space-y-4">
                      <h3 className="text-xs font-semibold text-muted-foreground tracking-wider flex items-center gap-2">
                        <HardDrive className="h-3.5 w-3.5" />
                        {t(
                          'token.configDialog.installationMethod',
                          'Installation Method'
                        )}
                      </h3>

                      <CommandGenerator
                        token={token}
                        account={account}
                        selectedTool={selectedTool}
                        selectedNode={selectedNode}
                        selectedModel={selectedModel}
                        isCompatible={isCompatible}
                        tempCommands={tempCommands}
                        setTempCommands={setTempCommands}
                        displayCommands={displayCommands}
                        copied={copied}
                        setCopied={setCopied}
                        isSingleLine={isSingleLine}
                        setIsSingleLine={setIsSingleLine}
                        onSuccess={() => onOpenChange(false)}
                      />
                    </div>
                  </>
                )}
              </div>
            </ScrollArea>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
