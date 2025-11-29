import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { CheckInStreakDto } from '@/hooks/useCheckInStreak';
import { useTranslation } from 'react-i18next';

interface AccountStreakSelectorProps {
  accounts: CheckInStreakDto[];
  selectedAccountId: string | 'all';
  onAccountChange: (accountId: string) => void;
}

export function AccountStreakSelector({
  accounts,
  selectedAccountId,
  onAccountChange,
}: AccountStreakSelectorProps) {
  const { t } = useTranslation();
  const accountsByProvider = accounts.reduce<Record<string, {
    providerName: string;
    accounts: CheckInStreakDto[];
  }>>((acc, account) => {
    if (!acc[account.provider_id]) {
      acc[account.provider_id] = {
        providerName: account.provider_name,
        accounts: [],
      };
    }
    acc[account.provider_id].accounts.push(account);
    return acc;
  }, {});

  return (
    <Select value={selectedAccountId} onValueChange={onAccountChange}>
      <SelectTrigger className="w-[200px]">
        <SelectValue placeholder={t('streaks.selectPlaceholder')} />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="all">{t('streaks.selectAll')}</SelectItem>
        {Object.entries(accountsByProvider).map(([providerId, group]) => (
          <SelectGroup key={providerId}>
            <SelectLabel>{group.providerName}</SelectLabel>
            {group.accounts.map((account) => (
              <SelectItem key={account.account_id} value={account.account_id}>
                <span className="block truncate" title={account.account_name}>
                  {account.account_name}
                </span>
              </SelectItem>
            ))}
          </SelectGroup>
        ))}
      </SelectContent>
    </Select>
  );
}
