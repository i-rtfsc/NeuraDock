import { useState, useEffect, useMemo } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import { 
  Copy, 
  Check, 
  AlertTriangle, 
  Terminal, 
  Globe, 
  Code2, 
  Sparkles, 
  Server, 
  ChevronRight,
  Settings,
  Plus,
  HardDrive, 
  Zap
} from 'lucide-react';
import type { TokenDto, AccountDto, ProviderNode } from '@/types/token';
import { cn } from '@/lib/utils';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Switch } from '@/components/ui/switch';

interface ConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  token: TokenDto | null;
  account: AccountDto | null;
}

type AITool = 'claude' | 'codex' | 'gemini';

export function ConfigDialog({
  open,
  onOpenChange,
  token,
  account,
}: ConfigDialogProps) {
  const { t } = useTranslation();
  const [selectedTool, setSelectedTool] = useState<AITool>('claude');
  const [selectedNode, setSelectedNode] = useState<string>('');
  const [selectedModel, setSelectedModel] = useState<string>('');
  const [tempCommands, setTempCommands] = useState('');
  const [copied, setCopied] = useState(false);
  const [compatibilityWarning, setCompatibilityWarning] = useState('');
  const [isCompatible, setIsCompatible] = useState(true);
  const [isSingleLine, setIsSingleLine] = useState(false);
  const modelLimitsEnabled = Boolean(token?.model_limits_enabled);

  // Fetch nodes for the account's provider
  const { data: nodes = [] } = useQuery<ProviderNode[]>({
    queryKey: ['provider-nodes', account?.provider_id],
    queryFn: () => invoke('get_provider_nodes', { providerId: account!.provider_id }),
    enabled: !!account && open,
  });

  // Fetch provider models from local cache when the token has no model restrictions
  const shouldLoadProviderModels = Boolean(token && !modelLimitsEnabled);
  const { data: providerModels = [], isFetching: isFetchingProviderModels } = useQuery<string[]>({
    queryKey: ['provider-models', account?.provider_id],
    queryFn: async () => {
      if (!account) return [];
      const models = await invoke<string[]>('get_cached_provider_models', {
        providerId: account.provider_id,
      });
      return models ?? [];
    },
    enabled: shouldLoadProviderModels,
    staleTime: 5 * 60 * 1000, // 5 minutes cache
  });
  const isModelListLoading = shouldLoadProviderModels && isFetchingProviderModels;

  // Determine available models based on token limits
  const availableModels = useMemo(() => {
    if (!token) return [];
    
    // 1. If model_limits_enabled is true (1), use model_limits_allowed
    if (modelLimitsEnabled) {
      return token.model_limits_allowed || [];
    }
    
    // 2. If model_limits_enabled is false (0), use all provider models
    return providerModels;
  }, [token, providerModels, modelLimitsEnabled]);

  // Filter models based on selected tool
  const filteredModels = useMemo(() => {
    if (!availableModels || availableModels.length === 0) return [];

    return availableModels.filter(m => {
      const lowerM = m.toLowerCase();
      if (selectedTool === 'claude') {
        return lowerM.includes('claude') || lowerM.includes('glm') || lowerM.includes('deepseek');
      } else if (selectedTool === 'codex') {
        return lowerM.includes('gpt');
      } else if (selectedTool === 'gemini') {
        return lowerM.includes('gemini');
      }
      return true;
    });
  }, [availableModels, selectedTool]);

  // Reset selected model when tool changes
  useEffect(() => {
    setSelectedModel('');
  }, [selectedTool]);

  // Check compatibility when tool or token changes
  useEffect(() => {
    if (!token || !open) return;

    if (isModelListLoading) {
      setIsCompatible(true);
      setCompatibilityWarning('');
      return;
    }

    if (availableModels.length === 0) {
      setIsCompatible(false);
      setCompatibilityWarning(t('token.configDialog.noCompatibleModels', '未找到可用模型'));
      return;
    }

    const modelsLower = availableModels.map(m => m.toLowerCase());

    if (selectedTool === 'claude') {
      const hasCompatible = modelsLower.some(m =>
        m.includes('claude') || m.includes('glm') || m.includes('deepseek')
      );

      if (!hasCompatible) {
        setIsCompatible(false);
        setCompatibilityWarning(t('token.noCompatibleModelsForClaude', '中转站可能不支持 Claude 兼容模型'));
      } else {
        setIsCompatible(true);
        setCompatibilityWarning('');
      }
    } else if (selectedTool === 'codex') {
      const hasGPT = modelsLower.some(m => m.includes('gpt'));

      if (!hasGPT) {
        setIsCompatible(false);
        setCompatibilityWarning(t('token.noGPTModels', '中转站可能不支持 GPT 模型'));
      } else {
        setIsCompatible(true);
        setCompatibilityWarning('');
      }
    } else if (selectedTool === 'gemini') {
      const hasGemini = modelsLower.some(m => m.includes('gemini'));

      if (!hasGemini) {
        setIsCompatible(false);
        setCompatibilityWarning(t('token.noGeminiModels', '中转站可能不支持 Gemini 模型'));
      } else {
        setIsCompatible(true);
        setCompatibilityWarning('');
      }
    } else {
      setIsCompatible(true);
      setCompatibilityWarning('');
    }
  }, [token, selectedTool, open, availableModels, t, isModelListLoading]);

  // Reset state when dialog opens
  useEffect(() => {
    if (open && token) {
      setTempCommands('');
      setCopied(false);
      setSelectedModel('');
      setIsSingleLine(false);
      setSelectedTool('claude'); // Reset to default

      if (nodes.length > 0) {
        setSelectedNode(nodes[0].base_url);
      }
    }
  }, [open, token]);

  useEffect(() => {
    if (nodes.length > 0 && !selectedNode) {
      setSelectedNode(nodes[0].base_url);
    }
  }, [nodes]);

  const configureGlobalMutation = useMutation({
    mutationFn: () => {
      if (selectedTool === 'claude') {
        return invoke<string>('configure_claude_global', {
          tokenId: token!.id,
          accountId: token!.account_id,
          baseUrl: selectedNode,
          model: selectedModel || null,
        });
      } else if (selectedTool === 'codex') {
        return invoke<string>('configure_codex_global', {
          tokenId: token!.id,
          accountId: token!.account_id,
          providerId: account!.provider_id,
          baseUrl: selectedNode,
          model: selectedModel || null,
        });
      } else {
        throw new Error("Not implemented");
      }
    },
    onSuccess: (message) => {
      toast.success(message);
      onOpenChange(false);
    },
    onError: (error: Error) => {
      toast.error(t('token.configError', 'Configuration failed: ') + error.message);
    },
  });

  const generateTempMutation = useMutation({
    mutationFn: () => {
      if (selectedTool === 'claude') {
        return invoke<string>('generate_claude_temp_commands', {
          tokenId: token!.id,
          accountId: token!.account_id,
          baseUrl: selectedNode,
          model: selectedModel || null,
        });
      } else if (selectedTool === 'codex') {
        return invoke<string>('generate_codex_temp_commands', {
          tokenId: token!.id,
          accountId: token!.account_id,
          providerId: account!.provider_id,
          baseUrl: selectedNode,
          model: selectedModel || null,
        });
      } else {
         throw new Error("Not implemented");
      }
    },
    onSuccess: (commands) => {
      setTempCommands(commands);
    },
    onError: (error: unknown) => {
      const message = error instanceof Error ? error.message : String(error);
      toast.error(t('token.generateError', 'Failed to generate commands: ') + message);
    },
  });

  const displayCommands = useMemo(() => {
    if (!tempCommands) return '';
    if (isSingleLine) {
      return tempCommands.trim().split('\n').filter(line => line.trim()).join(' && ');
    }
    return tempCommands;
  }, [tempCommands, isSingleLine]);

  const handleCopyCommands = async () => {
    try {
      await navigator.clipboard.writeText(displayCommands);
      setCopied(true);
      toast.success(t('token.configDialog.copied', 'Commands copied to clipboard'));
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      toast.error(t('token.copyError', 'Failed to copy'));
    }
  };

  // Reset generated commands when tool/model/node changes to avoid stale instructions
  useEffect(() => {
    if (!open) return;
    setTempCommands('');
    setCopied(false);
    setIsSingleLine(false);
  }, [selectedTool, selectedModel, selectedNode, open]);

  const handleManageNodes = () => {
    toast.info(t('token.manageNodesHint', 'Go to Settings > Nodes to manage your API endpoints.'));
  };

  if (!token || !account) return null;

  const toolOptions = [
    {
      id: 'claude',
      name: t('token.configDialog.tools.claude', 'Claude Code'),
      icon: Terminal,
      desc: t('token.configDialog.tools.claudeDesc', 'Anthropic CLI'),
    },
    {
      id: 'codex',
      name: t('token.configDialog.tools.codex', 'Codex'),
      icon: Code2,
      desc: t('token.configDialog.tools.codexDesc', 'OpenAI Compatible'),
    },
    {
      id: 'gemini',
      name: t('token.configDialog.tools.gemini', 'Gemini'),
      icon: Sparkles,
      desc: t('token.configDialog.tools.geminiDesc', 'Google AI'),
      disabled: true,
    },
  ];

  const currentTool = toolOptions.find(t => t.id === selectedTool);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl p-0 gap-0 overflow-hidden h-[600px] flex flex-col bg-zinc-50/50 dark:bg-zinc-950/50 backdrop-blur-xl">
        <div className="flex h-full">
          {/* Left Sidebar - Tool Selection */}
          <div className="w-64 bg-background/80 border-r flex flex-col shrink-0 backdrop-blur-sm">
            <div className="p-5 border-b">
              <h2 className="font-semibold text-xs text-muted-foreground uppercase tracking-wider mb-1">
                {t('token.selectTool', 'Integrations')}
              </h2>
              <p className="text-[10px] text-muted-foreground/70">
                {t('token.configDialog.toolDescription', 'Select the tool you want to configure')}
              </p>
            </div>
            <ScrollArea className="flex-1">
              <div className="p-3 space-y-1">
                {toolOptions.map((tool) => {
                  const Icon = tool.icon;
                  const isSelected = selectedTool === tool.id;
                  return (
                    <button
                      key={tool.id}
                      onClick={() => !tool.disabled && setSelectedTool(tool.id as AITool)}
                      disabled={tool.disabled}
                      className={cn(
                        "w-full flex items-center gap-3 px-3 py-3 rounded-xl text-left transition-all duration-200",
                        isSelected
                          ? "bg-primary text-primary-foreground shadow-md shadow-primary/20"
                          : "text-muted-foreground hover:bg-muted/80 hover:text-foreground",
                        tool.disabled && "opacity-50 cursor-not-allowed"
                      )}
                    >
                      <div className={cn(
                        "p-2 rounded-lg shrink-0 transition-colors",
                        isSelected ? "bg-primary-foreground/20 text-primary-foreground" : "bg-muted text-muted-foreground"
                      )}>
                        <Icon className="h-4 w-4" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="font-medium text-sm">{tool.name}</div>
                        <div className={cn("text-xs truncate opacity-80", isSelected ? "text-primary-foreground/80" : "text-muted-foreground")}>
                          {tool.desc}
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
                <span className="font-medium text-foreground truncate max-w-[120px]">{token.name}</span>
              </div>
            </div>
          </div>

          {/* Right Content - Configuration */}
          <div className="flex-1 flex flex-col min-w-0 bg-background/50">
            <DialogHeader className="px-8 py-6 border-b shrink-0 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <DialogTitle className="flex items-center gap-3 text-2xl font-bold tracking-tight">
                    {currentTool?.name}
                    {selectedTool === 'gemini' && (
                      <Badge variant="secondary" className="text-xs font-normal">
                        {t('token.configDialog.comingSoon', 'Coming Soon')}
                      </Badge>
                    )}
                  </DialogTitle>
                  <p className="text-sm text-muted-foreground">
                    {t('token.configDialog.subtitle', 'Configure connection settings and credentials.')}
                  </p>
                </div>
                <div className="flex flex-col items-end gap-1">
                  <Badge variant="outline" className="gap-1.5 py-1 px-2 text-xs font-medium bg-background/50">
                    <Server className="h-3 w-3 text-primary" />
                    <span className="opacity-70">{t('token.configDialog.provider', 'Provider')}:</span>
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
                      <p className="text-sm">{t('token.geminiSupportComingSoon', 'Support for Gemini is currently under development.')}</p>
                    </div>
                  </div>
                ) : (
                  <>
                    {/* Compatibility Alert */}
                    {compatibilityWarning && (
                      <Alert variant={isCompatible ? "default" : "destructive"} className="animate-in fade-in slide-in-from-top-2">
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
                        <div className="space-y-2">
                          <div className="flex items-center justify-between">
                            <Label className="text-xs font-medium">
                              {t('token.configDialog.apiEndpoint', 'API Endpoint')}
                            </Label>
                            <Button 
                              variant="ghost" 
                              size="sm" 
                              className="h-5 px-2 text-[10px] text-primary hover:text-primary/80 hover:bg-primary/10"
                              onClick={handleManageNodes}
                            >
                              <Plus className="h-3 w-3 mr-1" />
                              {t('token.configDialog.manageNodes', 'Manage Nodes')}
                            </Button>
                          </div>
                          <Select value={selectedNode} onValueChange={setSelectedNode}>
                            <SelectTrigger className="h-10 bg-background/50">
                              <SelectValue placeholder={t('token.chooseNode', 'Select an endpoint...')} />
                            </SelectTrigger>
                            <SelectContent>
                              {nodes.map((node) => (
                                <SelectItem key={node.id} value={node.base_url}>
                                  <div className="flex flex-col py-0.5">
                                    <span className="font-medium">{node.name}</span>
                                    <span className="text-[10px] text-muted-foreground">{node.base_url}</span>
                                  </div>
                                </SelectItem>
                              ))}
                              <Separator className="my-1" />
                              <Button 
                                variant="ghost" 
                                className="w-full justify-start h-8 px-2 text-xs font-normal text-muted-foreground hover:text-primary"
                                onClick={(e) => {
                                  e.preventDefault();
                                  handleManageNodes();
                                }}
                              >
                                <Settings className="h-3 w-3 mr-2" />
                                {t('token.configDialog.manageNodes', 'Manage Nodes...')}
                              </Button>
                            </SelectContent>
                          </Select>
                        </div>

                        <div className="space-y-2">
                          <Label className="text-xs font-medium">
                            {t('token.configDialog.defaultModel', 'Default Model')}
                          </Label>
                          <Select value={selectedModel} onValueChange={setSelectedModel} disabled={isModelListLoading}>
                            <SelectTrigger className="h-10 bg-background/50">
                              <SelectValue placeholder={
                                isModelListLoading 
                                  ? t('token.configDialog.loadingModels', 'Loading models...') 
                                  : t('token.configDialog.autoDetect', 'Auto-detect')
                              } />
                            </SelectTrigger>
                            <SelectContent>
                              {filteredModels.length > 0 ? (
                                filteredModels.map((model) => (
                                  <SelectItem key={model} value={model}>{model}</SelectItem>
                                ))
                              ) : (
                                <div className="p-3 text-xs text-muted-foreground text-center">
                                  {isModelListLoading 
                                    ? t('token.configDialog.loadingModels', 'Loading models...') 
                                    : t('token.configDialog.noSpecificModels', 'No specific models found')}
                                </div>
                              )}
                            </SelectContent>
                          </Select>
                        </div>
                      </div>
                    </div>

                    <Separator />

                    {/* Configuration Mode Tabs */}
                    <div className="space-y-4">
                      <h3 className="text-xs font-semibold text-muted-foreground tracking-wider flex items-center gap-2">
                        <HardDrive className="h-3.5 w-3.5" />
                        {t('token.configDialog.installationMethod', 'Installation Method')}
                      </h3>

                      <Tabs defaultValue="global" className="w-full">
                        <TabsList className="grid w-full grid-cols-2 h-11 bg-muted/50 p-1">
                          <TabsTrigger value="global" className="data-[state=active]:bg-background data-[state=active]:shadow-sm transition-all">
                            {t('token.configDialog.globalConfig', 'Global Config')}
                          </TabsTrigger>
                          <TabsTrigger value="temp" className="data-[state=active]:bg-background data-[state=active]:shadow-sm transition-all">
                            {t('token.configDialog.tempSession', 'Temporary Session')}
                          </TabsTrigger>
                        </TabsList>
                        
                        <TabsContent value="global" className="mt-4 space-y-3 animate-in fade-in slide-in-from-bottom-2">
                          <div className="rounded-lg bg-primary/5 border border-primary/10 p-3">
                            <div className="flex gap-2.5 items-start">
                              <div className="p-1.5 rounded-lg bg-primary/10 text-primary shrink-0">
                                <Globe className="h-3.5 w-3.5" />
                              </div>
                              <div className="flex-1 min-w-0">
                                <p className="text-xs text-muted-foreground leading-relaxed">
                                  {t('token.configDialog.globalConfigDesc')}
                                </p>
                                <p className="text-[10px] text-muted-foreground/70 mt-1 font-mono">
                                  {selectedTool === 'claude'
                                    ? '~/.claude/settings.json'
                                    : '~/.codex/config.toml'}
                                </p>
                              </div>
                            </div>
                          </div>
                          <Button
                            className="w-full h-10 text-sm font-medium shadow-lg shadow-primary/20"
                            onClick={() => configureGlobalMutation.mutate()}
                            disabled={!selectedNode || configureGlobalMutation.isPending || !isCompatible}
                          >
                            {configureGlobalMutation.isPending
                              ? t('common.configuring', 'Installing Configuration...')
                              : t('token.configDialog.installConfig', 'Install Configuration')}
                          </Button>
                        </TabsContent>
                        
                        <TabsContent value="temp" className="mt-4 space-y-3 animate-in fade-in slide-in-from-bottom-2">
                          <div className="rounded-lg bg-muted/30 border border-border/50 p-3">
                            <div className="flex gap-2.5 items-start">
                              <div className="p-1.5 rounded-lg bg-muted text-muted-foreground shrink-0">
                                <Terminal className="h-3.5 w-3.5" />
                              </div>
                              <div className="flex-1 min-w-0">
                                <p className="text-xs text-muted-foreground leading-relaxed">
                                  {t('token.configDialog.tempSessionDesc')}
                                </p>
                              </div>
                            </div>
                          </div>

                          {!tempCommands ? (
                            <Button
                              className="w-full h-10 text-sm font-medium"
                              variant="outline"
                              onClick={() => generateTempMutation.mutate()}
                              disabled={!selectedNode || generateTempMutation.isPending}
                            >
                              {generateTempMutation.isPending
                                ? t('common.generating', 'Generating Commands...')
                                : t('token.configDialog.generateCommands', 'Generate Export Commands')}
                            </Button>
                          ) : (
                            <div className="space-y-2">
                              <div className="flex items-center justify-between px-0.5">
                                <div className="flex items-center gap-2">
                                  <Switch 
                                    id="single-line-mode" 
                                    checked={isSingleLine} 
                                    onCheckedChange={setIsSingleLine}
                                    className="scale-90"
                                  />
                                  <Label htmlFor="single-line-mode" className="text-xs font-medium cursor-pointer">
                                    {isSingleLine ? t('token.configDialog.singleLine', '单行命令') : t('token.configDialog.multiLine', '多行命令')}
                                  </Label>
                                </div>
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  className="h-6 px-2 text-[10px] text-muted-foreground hover:text-foreground"
                                  onClick={() => setTempCommands('')}
                                >
                                  {t('token.configDialog.clear', '清除')}
                                </Button>
                              </div>
                              
                              <div className="relative rounded-lg border bg-zinc-950 shadow-inner overflow-hidden group">
                                <div className="absolute top-0 left-0 right-0 h-7 bg-zinc-900/50 flex items-center px-2.5 gap-1.5 border-b border-zinc-800 justify-between">
                                  <div className="flex gap-1.5">
                                    <div className="h-2 w-2 rounded-full bg-red-500/20" />
                                    <div className="h-2 w-2 rounded-full bg-yellow-500/20" />
                                    <div className="h-2 w-2 rounded-full bg-green-500/20" />
                                  </div>
                                  <Button
                                    size="sm"
                                    variant="ghost"
                                    className="h-5 px-1.5 text-[10px] text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800"
                                    onClick={handleCopyCommands}
                                  >
                                    {copied ? <Check className="h-3 w-3 mr-1" /> : <Copy className="h-3 w-3 mr-1" />}
                                    {copied ? t('token.configDialog.copied', '已复制') : t('token.configDialog.copy', '复制')}
                                  </Button>
                                </div>
                                <Textarea
                                  value={displayCommands}
                                  readOnly
                                  className={cn(
                                    "w-full resize-none bg-transparent font-mono text-[11px] text-zinc-300 border-none focus-visible:ring-0 px-3 pt-9 pb-3 leading-relaxed selection:bg-primary/30",
                                    isSingleLine ? "min-h-[70px] whitespace-nowrap overflow-x-auto" : "min-h-[100px]"
                                  )}
                                />
                              </div>
                            </div>
                          )}
                        </TabsContent>
                      </Tabs>
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
