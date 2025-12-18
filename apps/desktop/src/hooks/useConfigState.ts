import { useState, useEffect, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import type { TokenDto, AccountDto, ProviderNode } from '@/types/token';

export type AITool = 'claude' | 'codex' | 'gemini';

interface UseConfigStateProps {
  open: boolean;
  token: TokenDto | null;
  account: AccountDto | null;
}

interface UseConfigStateReturn {
  // Tool selection
  selectedTool: AITool;
  setSelectedTool: (tool: AITool) => void;

  // Node selection
  selectedNode: string;
  setSelectedNode: (node: string) => void;
  nodes: ProviderNode[];

  // Model selection
  selectedModel: string;
  setSelectedModel: (model: string) => void;
  availableModels: string[];
  filteredModels: string[];
  isModelListLoading: boolean;

  // Commands state
  tempCommands: string;
  setTempCommands: (commands: string) => void;
  copied: boolean;
  setCopied: (copied: boolean) => void;
  isSingleLine: boolean;
  setIsSingleLine: (singleLine: boolean) => void;
  displayCommands: string;

  // Compatibility
  compatibilityWarning: string;
  isCompatible: boolean;

  // Token info
  modelLimitsEnabled: boolean;
}

export function useConfigState({
  open,
  token,
  account,
}: UseConfigStateProps): UseConfigStateReturn {
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

  // Compute display commands based on single-line mode
  const displayCommands = useMemo(() => {
    if (!tempCommands) return '';
    if (isSingleLine) {
      return tempCommands.trim().split('\n').filter(line => line.trim()).join(' && ');
    }
    return tempCommands;
  }, [tempCommands, isSingleLine]);

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
      setCompatibilityWarning(t('token.configDialog.noCompatibleModels', 'No available models found'));
      return;
    }

    const modelsLower = availableModels.map(m => m.toLowerCase());
    let hasCompatible = false;

    if (selectedTool === 'claude') {
      hasCompatible = modelsLower.some(m =>
        m.includes('claude') || m.includes('glm') || m.includes('deepseek')
      );
    } else if (selectedTool === 'codex') {
      hasCompatible = modelsLower.some(m => m.includes('gpt'));
    } else if (selectedTool === 'gemini') {
      hasCompatible = modelsLower.some(m => m.includes('gemini'));
    }

    if (!hasCompatible) {
      setIsCompatible(false);
      if (modelLimitsEnabled) {
        setCompatibilityWarning(t('token.tokenRestrictedNoCompatibleModels', 'This token is restricted to specific models and does not support the selected tool.'));
      } else {
        setCompatibilityWarning(t('token.providerNoCompatibleModels', 'This provider does not support models compatible with the selected tool.'));
      }
    } else {
      setIsCompatible(true);
      setCompatibilityWarning('');
    }
  }, [token, selectedTool, open, availableModels, t, isModelListLoading, modelLimitsEnabled]);

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
  }, [open, token, nodes]);

  // Auto-select first node if none selected
  useEffect(() => {
    if (nodes.length > 0 && !selectedNode) {
      setSelectedNode(nodes[0].base_url);
    }
  }, [nodes, selectedNode]);

  // Reset generated commands when tool/model/node changes to avoid stale instructions
  useEffect(() => {
    if (!open) return;
    setTempCommands('');
    setCopied(false);
    setIsSingleLine(false);
  }, [selectedTool, selectedModel, selectedNode, open]);

  return {
    selectedTool,
    setSelectedTool,
    selectedNode,
    setSelectedNode,
    nodes,
    selectedModel,
    setSelectedModel,
    availableModels,
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
  };
}
