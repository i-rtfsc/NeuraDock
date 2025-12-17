import { useState, useEffect, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import type { TokenDto, AccountDto, ProviderNode } from '@/types/token';

type AITool = 'claude' | 'codex' | 'gemini';

interface UseConfigDialogStateProps {
  token: TokenDto | null;
  account: AccountDto | null;
  open: boolean;
}

export function useConfigDialogState({ token, account, open }: UseConfigDialogStateProps) {
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
    staleTime: 5 * 60 * 1000,
  });
  const isModelListLoading = shouldLoadProviderModels && isFetchingProviderModels;

  // Determine available models based on token limits
  const availableModels = useMemo(() => {
    if (!token) return [];
    if (modelLimitsEnabled) {
      return token.model_limits_allowed || [];
    }
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

    setIsCompatible(hasCompatible);
    if (!hasCompatible) {
      setCompatibilityWarning(
        t('token.configDialog.noCompatibleModelsForTool', 
          'No compatible models for selected tool')
      );
    } else {
      setCompatibilityWarning('');
    }
  }, [selectedTool, availableModels, token, open, t, isModelListLoading]);

  // Auto-select first node and model
  useEffect(() => {
    if (nodes.length > 0 && !selectedNode) {
      setSelectedNode(nodes[0].url);
    }
  }, [nodes, selectedNode]);

  useEffect(() => {
    if (filteredModels.length > 0 && !selectedModel && open) {
      setSelectedModel(filteredModels[0]);
    }
  }, [filteredModels, selectedModel, open]);

  return {
    selectedTool,
    setSelectedTool,
    selectedNode,
    setSelectedNode,
    selectedModel,
    setSelectedModel,
    tempCommands,
    setTempCommands,
    copied,
    setCopied,
    compatibilityWarning,
    isCompatible,
    isSingleLine,
    setIsSingleLine,
    modelLimitsEnabled,
    nodes,
    availableModels,
    filteredModels,
    isModelListLoading,
  };
}
