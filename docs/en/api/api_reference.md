# API Reference

## Overview

NeuraDock uses **Tauri IPC** for communication between the React frontend and Rust backend. All commands are type-safe, with TypeScript bindings auto-generated via **tauri-specta**.

## Importing Commands

```typescript
import { commands } from '@/lib/tauri';

// Call a command
const accounts = await commands.getAllAccounts(false);
```

---

## Account Commands

### `createAccount`

Create a new account.

```typescript
interface CreateAccountInput {
  name: string;
  provider_id: string;
  cookies: Record<string, string>;
  api_user?: string;
  auto_checkin_enabled?: boolean;
  auto_checkin_hour?: number;
  auto_checkin_minute?: number;
}

async function createAccount(input: CreateAccountInput): Promise<AccountDto>
```

**Example:**
```typescript
const account = await commands.createAccount({
  name: 'user@example.com',
  provider_id: 'anyrouter',
  cookies: { session: 'token_value' },
  api_user: 'user_123',
  auto_checkin_enabled: true,
  auto_checkin_hour: 8,
  auto_checkin_minute: 0,
});
```

### `updateAccount`

Update an existing account.

```typescript
interface UpdateAccountInput {
  account_id: string;
  name?: string;
  cookies?: Record<string, string>;
  api_user?: string;
  auto_checkin_enabled?: boolean;
  auto_checkin_hour?: number;
  auto_checkin_minute?: number;
}

async function updateAccount(input: UpdateAccountInput): Promise<AccountDto>
```

### `deleteAccount`

Delete an account by ID.

```typescript
async function deleteAccount(accountId: string): Promise<void>
```

### `toggleAccount`

Enable or disable an account.

```typescript
async function toggleAccount(accountId: string, enabled: boolean): Promise<AccountDto>
```

### `getAllAccounts`

Get all accounts, optionally filtered by enabled status.

```typescript
async function getAllAccounts(enabledOnly: boolean): Promise<AccountListDto[]>
```

**Response:**
```typescript
interface AccountListDto {
  id: string;
  name: string;
  provider_id: string;
  provider_name: string;
  enabled: boolean;
  auto_checkin_enabled: boolean;
  auto_checkin_hour: number;
  auto_checkin_minute: number;
  last_check_in_at?: string;
  created_at: string;
}
```

### `getAccountDetail`

Get detailed account information including credentials.

```typescript
async function getAccountDetail(accountId: string): Promise<AccountDetailDto>
```

**Response:**
```typescript
interface AccountDetailDto {
  id: string;
  name: string;
  provider_id: string;
  provider_name: string;
  cookies: Record<string, string>;
  cookie_count: number;
  api_user?: string;
  enabled: boolean;
  auto_checkin_enabled: boolean;
  auto_checkin_hour: number;
  auto_checkin_minute: number;
  quota?: number;
  used_quota?: number;
  remaining?: number;
  last_check_in_at?: string;
  last_balance_check_at?: string;
  created_at: string;
  updated_at: string;
}
```

---

## Check-In Commands

### `executeCheckIn`

Execute a check-in for a single account.

```typescript
async function executeCheckIn(accountId: string): Promise<CheckInResultDto>
```

**Response:**
```typescript
interface CheckInResultDto {
  success: boolean;
  message: string;
  user_info?: UserInfoDto;
}

interface UserInfoDto {
  email?: string;
  quota?: number;
  used?: number;
  remaining?: number;
}
```

### `executeBatchCheckIn`

Execute check-ins for multiple accounts.

```typescript
async function executeBatchCheckIn(accountIds: string[]): Promise<BatchCheckInResultDto>
```

**Response:**
```typescript
interface BatchCheckInResultDto {
  total: number;
  success: number;
  failed: number;
  results: CheckInResultItemDto[];
}

interface CheckInResultItemDto {
  account_id: string;
  account_name: string;
  success: boolean;
  message: string;
}
```

---

## Balance Commands

### `fetchAccountBalance`

Fetch balance for an account (uses cache if fresh).

```typescript
async function fetchAccountBalance(accountId: string): Promise<BalanceDto>
```

**Response:**
```typescript
interface BalanceDto {
  quota?: number;
  used_quota?: number;
  remaining?: number;
  last_check_at?: string;
  is_fresh: boolean;
}
```

### `fetchAccountsBalances`

Fetch balances for multiple accounts.

```typescript
async function fetchAccountsBalances(accountIds: string[]): Promise<BatchBalanceResultDto>
```

---

## Import/Export Commands

### `importAccountFromJson`

Import a single account from JSON.

```typescript
async function importAccountFromJson(jsonData: string): Promise<AccountDto>
```

### `importAccountsBatch`

Import multiple accounts from JSON array.

```typescript
async function importAccountsBatch(jsonData: string): Promise<BatchImportResultDto>
```

**Response:**
```typescript
interface BatchImportResultDto {
  total: number;
  imported: number;
  failed: number;
  errors: string[];
}
```

### `exportAccountsToJson`

Export accounts to JSON.

```typescript
interface ExportOptions {
  account_ids?: string[];
  include_credentials: boolean;
}

async function exportAccountsToJson(options: ExportOptions): Promise<string>
```

### `updateAccountsBatch`

Batch update account credentials.

```typescript
interface BatchUpdateInput {
  accounts: AccountUpdateItem[];
  create_if_not_exists: boolean;
}

interface AccountUpdateItem {
  name: string;
  provider: string;
  cookies: Record<string, string>;
  api_user?: string;
}

async function updateAccountsBatch(input: BatchUpdateInput): Promise<BatchUpdateResult>
```

**Response:**
```typescript
interface BatchUpdateResult {
  updated: number;
  created: number;
  failed: number;
  results: UpdateItemResult[];
}

interface UpdateItemResult {
  name: string;
  action: 'updated' | 'created' | 'failed';
  message?: string;
}
```

---

## Provider Commands

### `getBuiltinProviders`

Get list of built-in service providers.

```typescript
async function getBuiltinProviders(): Promise<ProviderDto[]>
```

**Response:**
```typescript
interface ProviderDto {
  id: string;
  name: string;
  base_url: string;
  requires_waf_bypass: boolean;
}
```

---

## Events

### Listening to Events

```typescript
import { events } from '@/lib/tauri';

// Listen to check-in progress
const unlisten = await events.checkInProgress.listen((event) => {
  console.log(event.payload);
});

// Clean up
unlisten();
```

### `checkInProgress`

Emitted during check-in execution.

```typescript
interface CheckInProgressEvent {
  account_id: string;
  status: 'started' | 'waf_bypass' | 'checking_in' | 'completed' | 'failed';
  message: string;
  progress: number; // 0-100
}
```

### `balanceUpdated`

Emitted when account balance is updated.

```typescript
interface BalanceUpdatedEvent {
  account_id: string;
  quota?: number;
  used_quota?: number;
  remaining?: number;
}
```

---

## Error Handling

All commands return `Result<T, String>`. Errors are returned as string messages.

```typescript
try {
  const account = await commands.createAccount(input);
} catch (error) {
  // error is a string message from backend
  console.error('Failed to create account:', error);
}
```

### Common Error Messages

| Error | Cause |
|-------|-------|
| "Account not found" | Invalid account ID |
| "Provider not found" | Invalid provider ID |
| "Invalid credentials" | Malformed cookies or API user |
| "WAF bypass failed" | Browser automation failed |
| "Network error" | Connection issues |
| "Session expired" | Cached session no longer valid |
