# Architecture Overview

## High-Level Architecture

NeuraDock is built using a **Domain-Driven Design (DDD)** approach with **CQRS (Command Query Responsibility Segregation)** pattern, implemented in Rust for the backend and React for the frontend.

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend (React)                          │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │    Pages    │ │ Components  │ │   TanStack Query        │ │
│  │ Dashboard   │ │ AccountCard │ │   (Server State)        │ │
│  │ Accounts    │ │ CheckInBtn  │ │                         │ │
│  │ Settings    │ │ Dialogs     │ │   Zustand (UI State)    │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘ │
└───────────────────────────┬─────────────────────────────────┘
                            │ Type-Safe IPC (tauri-specta)
┌───────────────────────────▼─────────────────────────────────┐
│                    Backend (Rust/Tauri)                      │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Presentation Layer                       │   │
│  │  commands.rs  │  events.rs  │  state.rs              │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │              Application Layer (CQRS)                 │   │
│  │  Commands  │  Queries  │  DTOs  │  Services          │   │
│  │  (writes)  │  (reads)  │        │  CheckInExecutor   │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │              Domain Layer (Core)                      │   │
│  │  Account Aggregate  │  CheckInJob Aggregate          │   │
│  │  Value Objects      │  Repository Traits             │   │
│  │  Domain Events      │  Domain Errors                 │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │              Infrastructure Layer                     │   │
│  │  SQLite Repository  │  HTTP Client  │  WAF Bypass    │   │
│  │  Browser Automation │  Notifications                 │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Layer Responsibilities

### Presentation Layer
- **Location**: `src-tauri/src/presentation/`
- **Purpose**: Handle Tauri IPC communication
- **Components**:
  - `commands.rs`: Tauri command handlers (IPC endpoints)
  - `events.rs`: Event definitions for frontend notifications
  - `state.rs`: Application state management (database, scheduler)

### Application Layer
- **Location**: `src-tauri/src/application/`
- **Purpose**: Orchestrate business operations
- **Components**:
  - `commands/`: Command handlers (write operations)
  - `queries/`: Query handlers (read operations)
  - `dtos/`: Data Transfer Objects for cross-layer communication
  - `services/`: Application services (CheckInExecutor, Scheduler)

### Domain Layer
- **Location**: `src-tauri/src/domain/`
- **Purpose**: Core business logic (framework-agnostic)
- **Components**:
  - `account/`: Account aggregate (root entity, value objects, repository trait)
  - `check_in/`: CheckInJob aggregate, Provider configuration
  - `shared/`: Shared value objects, ID types, errors
  - `events/`: Domain events

### Infrastructure Layer
- **Location**: `src-tauri/src/infrastructure/`
- **Purpose**: External concerns and implementations
- **Components**:
  - `persistence/`: SQLite repository implementations
  - `http/`: HTTP client, WAF bypass logic
  - `browser/`: Browser automation (chromiumoxide)
  - `notification/`: Notification services
  - `security/`: Encryption (placeholder)

## Key Design Patterns

### Repository Pattern
Domain layer defines repository traits (interfaces):
```rust
// domain/account/repository.rs
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError>;
    async fn save(&self, account: &Account) -> Result<(), DomainError>;
    async fn delete(&self, id: &AccountId) -> Result<(), DomainError>;
    // ...
}
```

Infrastructure implements these traits:
```rust
// infrastructure/persistence/repositories/account_repo.rs
impl AccountRepository for SqliteAccountRepository {
    // Implementation using sqlx
}
```

### Aggregate Pattern
Each aggregate is a consistency boundary:
- **Account**: Manages account state, credentials, auto check-in config
- **CheckInJob**: Represents a check-in execution with status transitions

### Value Objects
Immutable, validated data:
- `AccountId`, `ProviderId`: Type-safe identifiers
- `Credentials`: Cookie storage with validation

### Type-Safe IPC
Using tauri-specta for automatic TypeScript binding generation:
```rust
#[tauri::command]
#[specta::specta]
pub async fn create_account(input: CreateAccountInput, state: State<'_, AppState>)
    -> Result<AccountDto, String> {
    // ...
}
```

## Data Flow

### Check-In Flow
```
1. Frontend: User clicks "Check In" button
        │
        ▼
2. IPC: execute_check_in(account_id) called via tauri-specta
        │
        ▼
3. Presentation: commands.rs receives request, loads Account & Provider
        │
        ▼
4. Application: CheckInExecutor created with HTTP client
        │
        ▼
5. Infrastructure: WAF bypass if needed (browser automation)
        │
        ▼
6. Infrastructure: HTTP request to provider API
        │
        ▼
7. Domain: Account.record_check_in() updates state
        │
        ▼
8. Infrastructure: Save to database
        │
        ▼
9. Presentation: Return result to frontend
        │
        ▼
10. Frontend: Update UI via TanStack Query invalidation
```

### Auto Check-In Scheduling
```
1. Startup: Scheduler initialized in AppState::new()
        │
        ▼
2. Load: scheduler.reload_schedules() loads enabled accounts
        │
        ▼
3. Schedule: tokio-cron-scheduler creates jobs for each account
        │
        ▼
4. Trigger: At scheduled time, job executes check-in
        │
        ▼
5. Update: Account state updated, result logged
```

## Database Schema

```sql
accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    cookies TEXT NOT NULL,        -- JSON HashMap
    api_user TEXT,
    enabled INTEGER DEFAULT 1,
    auto_checkin_enabled INTEGER DEFAULT 0,
    auto_checkin_hour INTEGER DEFAULT 8,
    auto_checkin_minute INTEGER DEFAULT 0,
    -- Balance cache
    quota REAL, used_quota REAL, remaining REAL,
    last_balance_check_at TEXT,
    -- Session cache
    session_token TEXT, session_expiry TEXT,
    -- Timestamps
    last_check_in_at TEXT,
    created_at TEXT, updated_at TEXT
)

check_in_jobs (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id),
    provider_id TEXT,
    status TEXT,                  -- pending/running/completed/failed
    message TEXT,
    started_at TEXT, completed_at TEXT, created_at TEXT
)

balance_history (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id),
    quota REAL, used_quota REAL, remaining REAL,
    recorded_at TEXT
)
```

## Security Architecture

**Current State** (Issues):
- Credentials stored in plaintext in SQLite
- API responses include sensitive cookies
- No CSP configuration in Tauri

**Target State** (Planned):
- AES-GCM encryption for credentials at rest
- Credentials excluded from API responses
- Proper Tauri security configuration

## Performance Considerations

1. **Balance Caching**: 1-hour cache to reduce API calls
2. **Session Caching**: Reduces browser automation overhead
3. **Query Optimization**: Indexes on common query patterns
4. **Lazy Loading**: Balance fetched only when stale

## Related Documents

- [ADR-001: Use React for Frontend](./adr/001-use-react.md)
- [ADR-002: Database Selection](./adr/002-database-selection.md)
- [ADR-003: DDD Architecture](./adr/003-ddd-architecture.md)
- [ADR-004: CQRS Pattern](./adr/004-cqrs-pattern.md)
- [ADR-005: WAF Bypass Strategy](./adr/005-waf-bypass-strategy.md)
