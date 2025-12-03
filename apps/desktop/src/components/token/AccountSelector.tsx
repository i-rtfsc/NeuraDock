import { Card, CardContent } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useTranslation } from 'react-i18next';
import type { AccountDto } from '@/types/token';

interface AccountSelectorProps {
  accounts: AccountDto[];
  selectedAccount: string | null;
  onSelectAccount: (accountId: string) => void;
}

export function AccountSelector({
  accounts,
  selectedAccount,
  onSelectAccount,
}: AccountSelectorProps) {
  const { t } = useTranslation();

  const handleValueChange = (value: string) => {
    onSelectAccount(value);
  };

  if (accounts.length === 0) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          <p>{t('token.noAccounts', 'No AnyRouter or AgentRouter accounts found.')}</p>
          <p className="text-sm mt-2">
            {t('token.addAccountHint', 'Please add an account in the "Accounts" page first.')}
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardContent className="pt-6">
        <div className="space-y-2">
          <Label htmlFor="account-select">
            {t('token.selectAccount', 'Select Account')}
          </Label>
          <Select value={selectedAccount || ''} onValueChange={handleValueChange}>
            <SelectTrigger id="account-select" className="w-full">
              <SelectValue placeholder={t('token.chooseAccount', 'Choose an account...')} />
            </SelectTrigger>
            <SelectContent className="max-w-[600px]">
              {accounts.map((account) => (
                <SelectItem key={account.id} value={account.id} className="w-full">
                  <div className="flex items-center gap-2 w-full">
                    <span className="font-medium truncate max-w-[300px]" title={account.name}>
                      {account.name}
                    </span>
                    <span className="text-xs text-muted-foreground flex-shrink-0">
                      ({account.provider_name})
                    </span>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </CardContent>
    </Card>
  );
}
