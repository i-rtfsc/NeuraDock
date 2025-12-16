import { useState, useEffect } from 'react';
import { useMutation } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import {
  Copy,
  Check,
  Terminal,
  Code2,
  ChevronRight,
  HardDrive,
  Zap
} from 'lucide-react';
import type { IndependentKeyDto } from '@/types/independentKey';
import { cn } from '@/lib/utils';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Switch } from '@/components/ui/switch';

interface IndependentKeyConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  keyData: IndependentKeyDto | null;
  defaultTool?: AITool;
}

type AITool = 'claude' | 'codex';

export function IndependentKeyConfigDialog({
  open,
  onOpenChange,
  keyData,
  defaultTool = 'claude',
}: IndependentKeyConfigDialogProps) {
  const { t } = useTranslation();
  const [selectedTool, setSelectedTool] = useState<AITool>('claude');
  const [selectedModel, setSelectedModel] = useState<string>('');
  const [tempCommands, setTempCommands] = useState('');
  const [copied, setCopied] = useState(false);
  const [isSingleLine, setIsSingleLine] = useState(false);

  // Reset state when dialog opens
  useEffect(() => {
    if (open && keyData) {
      setTempCommands('');
      setCopied(false);
      setSelectedModel('');
      setIsSingleLine(false);
      setSelectedTool(defaultTool); // Use provided default tool
    }
  }, [open, keyData, defaultTool]);

  const configureGlobalMutation = useMutation({
    mutationFn: () => {
      if (selectedTool === 'claude') {
        return invoke<string>('configure_independent_key_claude', {
          keyId: keyData!.id,
          model: selectedModel || null,
        });
      } else if (selectedTool === 'codex') {
        return invoke<string>('configure_independent_key_codex', {
          keyId: keyData!.id,
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
        return invoke<string>('generate_independent_key_claude_temp', {
          keyId: keyData!.id,
          model: selectedModel || null,
        });
      } else if (selectedTool === 'codex') {
        return invoke<string>('generate_independent_key_codex_temp', {
          keyId: keyData!.id,
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

  const displayCommands = tempCommands ? (
    isSingleLine
      ? tempCommands.trim().split('\n').filter(line => line.trim()).join(' && ')
      : tempCommands
  ) : '';

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

  // Reset generated commands when tool/model changes
  useEffect(() => {
    if (!open) return;
    setTempCommands('');
    setCopied(false);
    setIsSingleLine(false);
  }, [selectedTool, selectedModel, open]);

  if (!keyData) return null;

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
  ];

  const currentTool = toolOptions.find(t => t.id === selectedTool);

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
                  <span className="font-medium text-primary truncate max-w-[180px]" title={keyData.name}>
                    {keyData.name}
                  </span>
                </div>
              </div>
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
                      onClick={() => setSelectedTool(tool.id as AITool)}
                      className={cn(
                        "w-full flex items-center gap-3 px-3 py-3 rounded-xl text-left transition-all duration-200",
                        isSelected
                          ? "bg-primary text-primary-foreground shadow-md shadow-primary/20"
                          : "text-muted-foreground hover:bg-muted/80 hover:text-foreground"
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
                <span className="font-medium text-foreground truncate max-w-[120px]">{keyData.name}</span>
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
                  </DialogTitle>
                  <p className="text-sm text-muted-foreground">
                    {t('token.configDialog.subtitle', 'Configure connection settings and credentials.')}
                  </p>
                </div>
              </div>
            </DialogHeader>

            <ScrollArea className="flex-1">
              <div className="p-8 space-y-6">
                {/* API Endpoint Info */}
                <div className="space-y-4">
                  <h3 className="text-xs font-semibold text-muted-foreground tracking-wider flex items-center gap-2">
                    <Zap className="h-3.5 w-3.5" />
                    {t('token.configDialog.connectionSetup', 'Connection Setup')}
                  </h3>

                  <Alert className="border-primary/20 bg-primary/5">
                    <AlertDescription className="text-sm">
                      <div className="font-medium mb-1">API Endpoint:</div>
                      <code className="text-xs bg-background/50 px-2 py-1 rounded">{keyData.base_url}</code>
                    </AlertDescription>
                  </Alert>

                  <div className="space-y-2">
                    <Label className="text-xs font-medium">
                      {t('token.configDialog.defaultModel', 'Default Model (Optional)')}
                    </Label>
                    <Input
                      value={selectedModel}
                      onChange={(e) => setSelectedModel(e.target.value)}
                      placeholder={t('token.configDialog.modelPlaceholder', 'Leave empty for auto-detect')}
                      className="h-10 bg-background/50"
                    />
                    <p className="text-[10px] text-muted-foreground">
                      {selectedTool === 'claude'
                        ? t('token.configDialog.claudeModelHint', 'e.g., claude-3-5-sonnet-20241022, claude-3-opus-20240229')
                        : t('token.configDialog.codexModelHint', 'e.g., gpt-4o, gpt-4-turbo')}
                    </p>
                  </div>
                </div>

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
                            <HardDrive className="h-3.5 w-3.5" />
                          </div>
                          <div className="flex-1 min-w-0">
                            <p className="text-xs text-muted-foreground leading-relaxed">
                              {t('token.configDialog.globalConfigDesc', 'Writes configuration to system files for persistent use')}
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
                        disabled={configureGlobalMutation.isPending}
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
                              {t('token.configDialog.tempSessionDesc', 'Generate export commands for the current terminal session')}
                            </p>
                          </div>
                        </div>
                      </div>

                      {!tempCommands ? (
                        <Button
                          className="w-full h-10 text-sm font-medium"
                          variant="outline"
                          onClick={() => generateTempMutation.mutate()}
                          disabled={generateTempMutation.isPending}
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
                                {isSingleLine ? t('token.configDialog.singleLine', 'Single Line') : t('token.configDialog.multiLine', 'Multi Line')}
                              </Label>
                            </div>
                            <Button
                              size="sm"
                              variant="ghost"
                              className="h-6 px-2 text-[10px] text-muted-foreground hover:text-foreground"
                              onClick={() => setTempCommands('')}
                            >
                              {t('token.configDialog.clear', 'Clear')}
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
                                {copied ? t('token.configDialog.copied', 'Copied') : t('token.configDialog.copy', 'Copy')}
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
              </div>
            </ScrollArea>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
