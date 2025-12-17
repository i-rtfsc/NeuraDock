# Architecture Overview

## High-Level Architecture

NeuraDock is built using a **Domain-Driven Design (DDD)** approach with **CQRS (Command Query Responsibility Segregation)** pattern, implemented in Rust for the backend and React for the frontend.

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend (React)                          │
│  ┌─────────────┐ ┌────────────┐ ┌─────────────────────────┐ │
│  │    Pages    │ │ Components  │ │   TanStack Query        │ │
│  │ Dashboard   │ │ AccountCard │ │   (Server State)        │ │
│  │ Accounts    │ │ CheckInBtn  │ │                         │ │
│  │ Settings    │ │ Dialogs     │ │   Zustand (UI State)    │ │
│  └────────────┘ └─────────────┘ └─────────────────────────┘ │
└───────────────────────────┬─────────────────────────────────┘
                            │ Type-Safe IPC (tauri-specta)
┌───────────────────────────▼─────────────────────────────────┐
│                    Backend (Rust/Tauri)                      │
│  ┌──────────────────────────────────────────────────────┐   │
│  │       Presentation Layer (neuradock-app/presentation) │   │
│  │  commands.rs  │  events.rs  │  state.rs              │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                                │
│  ──────────────────────────▼───────────────────────────┐   │
│  │      Application Layer (neuradock-app/application)    │   │
│  │  Commands  │  Queries  │  DTOs  │  Services          │   │
│  │  (writes)  │  (reads)  │        │  CheckInExecutor   │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │         Domain Layer (neuradock-domain)               │   │
│  │  Account │ Balance │ Session │ CheckIn │ Notification│   │
│  │  Value Objects │ Repository Traits │ Domain Events   │   │
│  └──────────────────────────┬──────────────────────────┘   │
│                             │                                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │    Infrastructure Layer (neuradock-infrastructure)    │   │
│  │  SQLite Repos │ HTTP Client │ WAF Bypass │ Notif Svc │   │
│  │  Browser Auto │ Encryption │ Event Bus │ Monitoring  │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Layer Responsibilities

### Multi-Crate Organization

NeuraDock uses a Rust workspace multi-crate architecture for better modularity and separation of concerns:

- **neuradock-app**: Application layer + Presentation layer
- **neuradock-domain**: Domain layer (Core business logic)
- **neuradock-infrastructure**: Infrastructure layer (External integrations)

### Presentation Layer
- **Location**: `src-tauri/crates/neuradock-app/src/presentation/`
- **Purpose**: Handle Tauri IPC communication
- **Components**:
  - `commands.rs`: Tauri command handlers (IPC endpoints)
  - `events.rs`: Event definitions for frontend notifications
  - `state.rs`: Application state management (database, scheduler)

### Application Layer
- **Location**: `src-tauri/crates/neuradock-app/src/application/`
- **Purpose**: Orchestrate business operations
- **Components**:
  - `commands/`: Command handlers (write operations)
  - `queries/`: Query handlers (read operations)
  - `dtos/`: Data transfer objects for cross-layer communication
  - `services/`: Application services (CheckInExecutor, Scheduler)
  - `event_handlers/`: Domain event handlers

### Domain Layer
- **Location**: `src-tauri/crates/neuradock-domain/src/`
- **Purpose**: Core business logic (framework-agnostic)
- **Components**:
  - `account/`: Account aggregate (root entity, value objects, repository trait)
  - `balance/`: Balance aggregate (balance tracking)
  - `check_in/`: CheckIn aggregate, Provider configuration
  - `session/`: Session aggregate (session management)
  - `notification/`: Notification aggregate (notification management)
  - `shared/`: Shared value objects, ID types, errors
  - `events/`: Domain events
  - `config/providers/`: Built-in relay definitions (seeded into the database on first launch)

### Infrastructure Layer
- **Location**: `src-tauri/crates/neuradock-infrastructure/src/`
- **Purpose**: External concerns and implementations
- **Components**:
  - `persistence/`: SQLite repository implementations
  - `http/`: HTTP client, WAF bypass logic
  - `browser/`: Browser automation (chromiumoxide)
  - `notification/`: Notification services (Feishu, Email, etc.)
  - `security/`: Encryption services
  - `events/`: Event bus implementation
  - `monitoring/`: Performance monitoring
  - `migrations/`: Database migrations

## Key Design Patterns

### Repository Pattern
Domain layer defines repository traits (interfaces):
```rust
// neuradock-domain/src/account/repository.rs
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError>;
    async fn save(&self, account: &Account) -> Result<(), DomainError>;
    async fn delete(&self, id: &AccountId) -> Result<(), DomainError>;
    // ...
}
```

Infrastructure layer implements these traits:
```rust
// neuradock-infrastructure/src/persistence/repositories/account_repo.rs
impl AccountRepository for SqliteAccountRepository {
    // Implemented using sqlx
}
```

### Aggregate Pattern
Each aggregate is a consistency boundary:
- **Account**: Manages account state, credentials, auto check-in configuration
- **Balance**: Tracks account balance and history
- **Session**: Manages session tokens and login state
- **CheckIn**: Represents a check-in execution and its state
- **Notification**: Manages notification channels and delivery

### Value Objects
Immutable, validated data:
- `AccountId`, `ProviderId`: Type-safe identifiers
- `Credentials`: Validated cookie storage

### Type-Safe IPC
Auto-generated TypeScript bindings using tauri-specta:
```rust
#[tauri::command]
#[specta::specta]
pub async fn create_account(input: CreateAccountInput, state: State<'_, AppState>)
    -> Result<AccountDto, String> {
    // ...
}
```

## Data Flow

### Check-in Flow
```
1. Frontend: User clicks "Check-in" button
        │
        ▼
2. IPC: Call execute_check_in(account_id) via tauri-specta
        │
        ▼
3. Presentation: commands.rs receives request, loads Account and Provider
        │
        ▼
4. Application: Create CheckInExecutor with HTTP client
        │
        ▼
5. Infrastructure: Perform WAF bypass if needed (browser automation)
        │
        ▼
6. Infrastructure: Send HTTP request to provider API
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
10. Frontend: Invalidate cache and refresh UI via TanStack Query
```

### Auto Check-in Scheduling
```
1. Startup: Initialize scheduler in AppState::new()
        │
        ▼
2. Load: scheduler.reload_schedules() loads enabled accounts
        │
        ▼
3. Schedule: tokio-cron-scheduler creates tasks for each account
        │
        ▼
4. Trigger: Execute check-in at scheduled time
        │
        ▼
5. Update: Account state updated, results logged
```

## Database Schema

```sql
-- Accounts table
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
    last_check_in_at TEXT,
    created_at TEXT, updated_at TEXT
)

-- Sessions table (separate management)
sessions (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id),
    session_token TEXT,
    last_login_at TEXT,
    expires_at TEXT,
    created_at TEXT
)

-- Balances table (separate management)
balances (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id),
    current_balance REAL,
    total_consumed REAL,
    total_income REAL,
    last_check_at TEXT,
    created_at TEXT, updated_at TEXT
)

-- Balance history table
balance_history (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id),
    current_balance REAL NOT NULL,
    total_consumed REAL NOT NULL,
    total_income REAL NOT NULL,
    recorded_at TEXT NOT NULL
)

-- Notification channels table
notification_channels (
    id TEXT PRIMARY KEY,
    channel_type TEXT NOT NULL,  -- feishu/email/telegram
    config TEXT NOT NULL,         -- JSON config
    enabled INTEGER DEFAULT 1,
    created_at TEXT
)

-- Providers table
providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    domain TEXT NOT NULL,
    login_path TEXT NOT NULL,
    sign_in_path TEXT,
    user_info_path TEXT NOT NULL,
    api_user_key TEXT NOT NULL,
    bypass_method TEXT,
    is_builtin INTEGER DEFAULT 0,
    created_at TEXT
)
```

## Security Architecture

**Current State** (Issues):
- Credentials stored in plaintext in SQLite
- API responses contain sensitive cookies
- Tauri CSP not configured

**Target State** (Planned):
- Encrypt static credentials using AES-GCM
- Exclude credentials from API responses
- Properly configure Tauri security settings

## Performance Considerations

1. **Balance Caching**: 1-hour cache reduces API calls
2. **Session Caching**: Reduces browser automation overhead
3. **Query Optimization**: Indexes for common query patterns
4. **Lazy Loading**: Fetch balance only when expired
5. **Event-Driven**: Async domain event processing for better responsiveness
6. **Connection Pooling**: SQLite connection pool for optimized database access

## Related Documentation

- [ADR-001: Using React for Frontend](./adr/001-use-react.md)
- [ADR-002: Database Selection](./adr/002-database-selection.md)
- [ADR-003: DDD Architecture](./adr/003-ddd-architecture.md)
- [ADR-004: CQRS Pattern](./adr/004-cqrs-pattern.md)
- [ADR-005: WAF Bypass Strategy](./adr/005-waf-bypass-strategy.md)
