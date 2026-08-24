import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands } from '@/lib/tauri';
const unwrap = <T,>(r: { status: 'ok'; data: T } | { status: 'error'; error: { message: string } }): T => { if (r.status === 'error') throw new Error(r.error.message); return r.data; };
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';

// Define types matching the Rust DTOs
export interface AiChatServiceDto {
  id: string;
  name: string;
  url: string;
  icon: string | null;
  is_builtin: boolean;
  is_enabled: boolean;
  sort_order: number;
}

export interface CreateAiChatServiceInput {
  name: string;
  url: string;
  icon: string | null;
}

export interface UpdateAiChatServiceInput {
  id: string;
  name: string;
  url: string;
  icon: string | null;
}

export interface ReorderAiChatServicesInput {
  service_ids: string[];
}

// Query key
const AI_CHAT_SERVICES_KEY = ['aiChatServices'];
const ENABLED_AI_CHAT_SERVICES_KEY = ['aiChatServices', 'enabled'];

/**
 * Hook for fetching all AI chat services
 */
export function useAiChatServices() {
  return useQuery({
    queryKey: AI_CHAT_SERVICES_KEY,
    queryFn: async (): Promise<AiChatServiceDto[]> => {
      return commands.listAiChatServices().then(unwrap);
    },
  });
}

/**
 * Hook for fetching only enabled AI chat services
 */
export function useEnabledAiChatServices() {
  return useQuery({
    queryKey: ENABLED_AI_CHAT_SERVICES_KEY,
    queryFn: async (): Promise<AiChatServiceDto[]> => {
      return commands.listEnabledAiChatServices().then(unwrap);
    },
  });
}

/**
 * Hook for creating a new AI chat service
 */
export function useCreateAiChatService() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (input: CreateAiChatServiceInput): Promise<AiChatServiceDto> => {
      return commands.createAiChatService(input).then(unwrap);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: AI_CHAT_SERVICES_KEY });
      toast.success(t('aiChat.serviceCreated'));
    },
    onError: (error: Error) => {
      toast.error(t('aiChat.createError', { message: error.message }));
    },
  });
}

/**
 * Hook for updating an AI chat service
 */
export function useUpdateAiChatService() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (input: UpdateAiChatServiceInput): Promise<AiChatServiceDto> => {
      return commands.updateAiChatService(input).then(unwrap);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: AI_CHAT_SERVICES_KEY });
      toast.success(t('aiChat.serviceUpdated'));
    },
    onError: (error: Error) => {
      toast.error(t('aiChat.updateError', { message: error.message }));
    },
  });
}

/**
 * Hook for deleting an AI chat service
 */
export function useDeleteAiChatService() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (id: string): Promise<boolean> => {
      return commands.deleteAiChatService(id).then(unwrap);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: AI_CHAT_SERVICES_KEY });
      toast.success(t('aiChat.serviceDeleted'));
    },
    onError: (error: Error) => {
      toast.error(t('aiChat.deleteError', { message: error.message }));
    },
  });
}

/**
 * Hook for toggling an AI chat service's enabled state
 */
export function useToggleAiChatService() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (id: string): Promise<AiChatServiceDto> => {
      return commands.toggleAiChatService(id).then(unwrap);
    },
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: AI_CHAT_SERVICES_KEY });
      const action = data.is_enabled ? t('aiChat.enabled') : t('aiChat.disabled');
      toast.success(t('aiChat.serviceToggled', { action }));
    },
    onError: (error: Error) => {
      toast.error(t('aiChat.toggleError', { message: error.message }));
    },
  });
}

/**
 * Hook for reordering AI chat services
 */
export function useReorderAiChatServices() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (input: ReorderAiChatServicesInput): Promise<boolean> => {
      return commands.reorderAiChatServices(input).then(unwrap);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: AI_CHAT_SERVICES_KEY });
      toast.success(t('aiChat.reorderSuccess'));
    },
    onError: (error: Error) => {
      toast.error(error.message);
    },
  });
}

/**
 * Hook for opening an AI chat service in a WebView
 */
export function useOpenAiChatWebview() {
  return useMutation({
    mutationFn: async (service: AiChatServiceDto): Promise<boolean> => {
      return commands.openAiChatWebview(service.id, service.name, service.url).then(unwrap);
    },
  });
}

/**
 * Hook for closing an AI chat WebView
 */
export function useCloseAiChatWebview() {
  return useMutation({
    mutationFn: async (serviceId: string): Promise<boolean> => {
      return commands.closeAiChatWebview(serviceId).then(unwrap);
    },
  });
}

/**
 * Hook for opening an AI chat service in the default browser
 */
export function useOpenAiChatInBrowser() {
  return useMutation({
    mutationFn: async (url: string): Promise<boolean> => {
      return commands.openAiChatInBrowser(url).then(unwrap);
    },
  });
}
