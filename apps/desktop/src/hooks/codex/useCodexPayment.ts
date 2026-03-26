import { useMutation } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import type {
  CodexPaymentIntervalDto,
  CodexPaymentLinkDto,
  CodexPaymentLinkRequestDto,
  CodexPaymentPlanDto,
} from '@/lib/tauri';
import { toast } from 'sonner';

interface GenerateCodexPaymentLinkArgs {
  accountId: string;
  request: CodexPaymentLinkRequestDto;
}

export interface CodexPaymentFormState {
  planType: CodexPaymentPlanDto;
  country: string;
  currency: string;
  workspaceName: string;
  seatQuantity: string;
  priceInterval: CodexPaymentIntervalDto;
}

export function useGenerateCodexPaymentLink() {
  const { t } = useTranslation();
  return useMutation({
    mutationFn: ({ accountId, request }: GenerateCodexPaymentLinkArgs): Promise<CodexPaymentLinkDto> =>
      invoke('generate_codex_payment_link', { accountId, request }),
    onSuccess: () => {
      toast.success(t('codex.toast.paymentLinkGenerated'));
    },
    onError: (e: any) => {
      toast.error(t('codex.toast.paymentLinkGenerateFailed', { message: e?.message ?? e }));
    },
  });
}
