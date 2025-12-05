import { useState, useEffect, useMemo } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import {
  Dialog,
  DialogContent,
  DialogDescription,
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import { Copy, Check, AlertTriangle } from 'lucide-react';
import type { TokenDto, AccountDto, ProviderNode } from '@/types/token';

interface ConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  token: TokenDto | null;
  account: AccountDto | null;
}

type AITool = 'claude' | 'codex' | 'gemini';
type ConfigMode = 'global' | 'temp';

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
  const [configMode, setConfigMode] = useState<ConfigMode>('global');
  const [tempCommands, setTempCommands] = useState('');
  const [copied, setCopied] = useState(false);
  const [compatibilityWarning, setCompatibilityWarning] = useState('');
  const [isCompatible, setIsCompatible] = useState(true);

  // Fetch nodes for the account's provider
  const { data: nodes = [] } = useQuery<ProviderNode[]>({
    queryKey: ['provider-nodes', account?.provider_id],
    queryFn: () => invoke('get_provider_nodes', { providerId: account!.provider_id }),
    enabled: !!account && open,
  });

  // Fetch provider models for compatibility check when model limits are disabled
  const { data: providerModels = [] } = useQuery<string[]>({
    queryKey: ['provider-models', account?.provider_id],
    queryFn: () => invoke('fetch_provider_models', {
      providerId: account!.provider_id,
      accountId: account!.id,
      forceRefresh: false,
    }),
    enabled: !!account && !!token && !token.model_limits_enabled && open,
    staleTime: 24 * 60 * 60 * 1000,
  });

  // Determine available models based on token limits or provider models
  const availableModels = useMemo(() => {
    if (!token) return [];
    return token.model_limits_enabled ? token.model_limits_allowed : providerModels;
  }, [token, providerModels]);

  // Filter models based on selected tool
  const filteredModels = useMemo(() => {
    if (!availableModels || availableModels.length === 0) return [];

    return availableModels.filter(m => {
      const lowerM = m.toLowerCase();
      if (selectedTool === 'claude') {
        return lowerM.includes('claude') || lowerM.includes('glm') || lowerM.includes('deepseek');
      } else if (selectedTool === 'codex') {
        return lowerM.includes('gpt');
      }
      // For Gemini, we might add filters later, but for now just show all or specific ones if we knew
      return true;
    });
  }, [availableModels, selectedTool]);

  // Reset selected model when tool changes
  useEffect(() => {
    setSelectedModel('');
  }, [selectedTool]);

  // Check compatibility when tool or token changes
  useEffect(() => {
    if (token && open) {
      if (availableModels.length === 0 && !token.model_limits_enabled) {
        // Still loading provider models
        setIsCompatible(true);
        setCompatibilityWarning('');
        return;
      }

      // For tokens without model limits, check differently
      const checkCompatibilityForUnrestrictedToken = (models: string[]) => {
        const modelsLower = models.map(m => m.toLowerCase());

        if (selectedTool === 'claude') {
          // For Claude Code, check if provider has compatible models
          const hasCompatible = modelsLower.some(m =>
            m.includes('claude') || m.includes('glm') || m.includes('deepseek') || m.includes('gemini')
          );

          if (!hasCompatible) {
            setIsCompatible(false);
            setCompatibilityWarning(
              t('token.noCompatibleModelsForClaude',
                'This provider does not support Claude-compatible models (Claude, GLM, DeepSeek, Gemini).')
            );
          } else {
            setIsCompatible(true);
            setCompatibilityWarning(
              t('token.noModelLimits', 'Note: This token has no model restrictions.')
            );
          }
        } else if (selectedTool === 'codex') {
          // For Codex, check if provider has GPT models
          const hasGPT = modelsLower.some(m =>
            m.includes('gpt') || m.includes('openai') || m.includes('o1')
          );

          if (!hasGPT) {
            setIsCompatible(true);
            setCompatibilityWarning(
              t('token.noGPTModels',
                'Note: This token has no model restrictions. This provider may not have GPT models, but Codex can work with OpenAI-compatible APIs.')
            );
          } else {
            setIsCompatible(true);
            setCompatibilityWarning(
              t('token.noModelLimits', 'Note: This token has no model restrictions.')
            );
          }
        } else if (selectedTool === 'gemini') {
           // Gemini compatibility check (placeholder)
           setIsCompatible(true);
           setCompatibilityWarning(t('token.geminiSupportComingSoon', 'Gemini configuration support is coming soon.'));
        }
      };

      // For tokens with model limits, use the original check
      if (!token.model_limits_enabled) {
        checkCompatibilityForUnrestrictedToken(availableModels);
      } else {
        invoke<[boolean, string]>('check_model_compatibility', {
          models: availableModels,
          tool: selectedTool,
        })
          .then(([compatible, warning]) => {
            setIsCompatible(compatible);
            setCompatibilityWarning(warning);
          })
          .catch((err) => {
            console.error('Failed to check compatibility:', err);
          });
      }
    }
  }, [token, selectedTool, open, availableModels, t]);

  // Reset state when dialog opens
  useEffect(() => {
    if (open) {
      setTempCommands('');
      setCopied(false);
      setConfigMode('global');
      setSelectedModel('');

      // Set default node when nodes load
      if (nodes.length > 0 && !selectedNode) {
        setSelectedNode(nodes[0].base_url);
      }

      // Auto-select tool based on token models
      if (token?.model_limits_allowed.some((m) =>
        m.toLowerCase().includes('claude') || m.toLowerCase().includes('glm') || m.toLowerCase().includes('deepseek')
      )) {
        setSelectedTool('claude');
      } else if (token?.model_limits_allowed.some((m) =>
        m.toLowerCase().includes('gpt') || m.toLowerCase().includes('openai')
      )) {
        setSelectedTool('codex');
      }
    }
  }, [open, token]);

  // Update selected node when nodes change
  useEffect(() => {
    if (nodes.length > 0 && !selectedNode) {
      setSelectedNode(nodes[0].base_url);
    }
  }, [nodes]);

  // Global configuration mutation
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
        throw new Error("Gemini configuration not implemented yet");
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

  // Generate temp commands mutation
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
         throw new Error("Gemini configuration not implemented yet");
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

  const handleCopyCommands = async () => {
    try {
      await navigator.clipboard.writeText(tempCommands);
      setCopied(true);
      toast.success(t('token.copied', 'Commands copied to clipboard'));
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      toast.error(t('token.copyError', 'Failed to copy'));
    }
  };

  if (!token || !account) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t('token.configureTitle', 'Configure AI Tool')}</DialogTitle>
          <DialogDescription>
            {t('token.configureDescription', 'Configure token')} "{token.name}" {t('token.forTools', 'for use with AI coding tools')}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6">
          {/* AI Tool Selector */}
          <div className="space-y-2">
            <Label>{t('token.selectTool', 'Select AI Tool')}</Label>
            <Select value={selectedTool} onValueChange={(val) => setSelectedTool(val as AITool)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="claude">Claude Code</SelectItem>
                <SelectItem value="codex">Codex (OpenAI)</SelectItem>
                <SelectItem value="gemini">{t('token.geminiComingSoon', 'Gemini (Coming Soon)')}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Compatibility Warning */}
          {compatibilityWarning && (
            <Alert variant={isCompatible ? "default" : "destructive"}>
              <AlertTriangle className="h-4 w-4" />
              <AlertDescription>{compatibilityWarning}</AlertDescription>
            </Alert>
          )}

          {/* Model Selector */}
          <div className="space-y-2">
            <Label>{t('token.selectModel', 'Select Model (Optional)')}</Label>
            <Select value={selectedModel} onValueChange={setSelectedModel} disabled={selectedTool === 'gemini'}>
              <SelectTrigger>
                <SelectValue placeholder={t('token.chooseModel', 'Choose a model (optional)...')} />
              </SelectTrigger>
              <SelectContent>
                 {filteredModels.length > 0 ? (
                    filteredModels.map((model) => (
                      <SelectItem key={model} value={model}>
                        {model}
                      </SelectItem>
                    ))
                 ) : (
                     <div className="p-2 text-sm text-muted-foreground text-center">
                        {t('token.noModelsAvailable', 'No compatible models found')}
                     </div>
                 )}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              {t('token.modelHint', 'Select a specific model to configure default model settings')}
            </p>
          </div>


          {/* Node Selector - For both Claude and Codex */}
          <div className="space-y-2">
            <Label>{t('token.selectNode', 'Select Node')}</Label>
            <Select value={selectedNode} onValueChange={setSelectedNode} disabled={selectedTool === 'gemini'}>
              <SelectTrigger>
                <SelectValue placeholder={t('token.chooseNode', 'Choose a node...')} />
              </SelectTrigger>
              <SelectContent>
                {nodes.map((node) => (
                  <SelectItem key={node.id} value={node.base_url}>
                    <div className="flex items-center gap-2">
                      <span>{node.name}</span>
                      <span className="text-xs text-muted-foreground">({node.base_url})</span>
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              {t('token.nodeHint', 'Manage nodes in the "Node Management" section above')}
            </p>
          </div>

          {/* Configuration Tabs with controlled state */}
          <Tabs value={configMode} onValueChange={(val) => setConfigMode(val as ConfigMode)} className="w-full">
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="global" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
                {t('token.globalConfig', 'Global Configuration')}
              </TabsTrigger>
              <TabsTrigger value="temp" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
                {t('token.tempConfig', 'Temporary Configuration')}
              </TabsTrigger>
            </TabsList>

            <TabsContent value="global" className="space-y-4 mt-4">
               {selectedTool === 'gemini' ? (
                 <div className="p-4 border rounded-md bg-muted text-center text-muted-foreground">
                    {t('token.geminiSupportComingSoon', 'Gemini configuration support is coming soon.')}
                 </div>
               ) : (
                  <>
                  <p className="text-sm text-muted-foreground">
                    {selectedTool === 'claude'
                      ? t('token.claudeGlobalDesc', 'This will write the configuration to ~/.claude/settings.json and apply globally.')
                      : t('token.codexGlobalDesc', 'This will write the configuration to ~/.codex/config.toml and ~/.codex/auth.json and apply globally.')}
                  </p>
                  <Button
                    className="w-full"
                    onClick={() => configureGlobalMutation.mutate()}
                    disabled={!selectedNode || configureGlobalMutation.isPending || !isCompatible}
                  >
                    {configureGlobalMutation.isPending
                      ? t('common.configuring', 'Configuring...')
                      : t('token.configureGlobally', 'Configure Globally')}
                  </Button>
                  </>
               )}
            </TabsContent>

            <TabsContent value="temp" className="space-y-4 mt-4">
               {selectedTool === 'gemini' ? (
                 <div className="p-4 border rounded-md bg-muted text-center text-muted-foreground">
                    {t('token.geminiSupportComingSoon', 'Gemini configuration support is coming soon.')}
                 </div>
               ) : (
                  <>
                  <p className="text-sm text-muted-foreground">
                    {t('token.tempConfigDesc', 'Generate export commands to use in your current terminal session only.')}
                  </p>
                  <Button
                    className="w-full"
                    variant="outline"
                    onClick={() => generateTempMutation.mutate()}
                    disabled={!selectedNode || generateTempMutation.isPending}
                  >
                    {generateTempMutation.isPending
                      ? t('common.generating', 'Generating...')
                      : t('token.generateCommands', 'Generate Commands')}
                  </Button>

                  {tempCommands && (
                    <div className="space-y-2">
                      <Textarea
                        value={tempCommands}
                        readOnly
                        className="font-mono text-sm"
                        rows={selectedTool === 'claude' ? 6 : 4}
                      />
                      <Button
                        variant="secondary"
                        className="w-full"
                        onClick={handleCopyCommands}
                      >
                        {copied ? (
                          <>
                            <Check className="mr-2 h-4 w-4" />
                            {t('common.copied', 'Copied!')}
                          </>
                        ) : (
                          <>
                            <Copy className="mr-2 h-4 w-4" />
                            {t('token.copyToClipboard', 'Copy to Clipboard')}
                          </>
                        )}
                      </Button>
                    </div>
                  )}
                  </>
               )}
            </TabsContent>
          </Tabs>
        </div>
      </DialogContent>
    </Dialog>
  );
}