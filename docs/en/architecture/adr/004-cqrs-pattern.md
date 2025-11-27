# ADR-004: CQRS Pattern

## Status
Accepted (Partially Implemented)

## Context
NeuraDock has different requirements for:
- **Commands** (writes): Create account, execute check-in, update settings
- **Queries** (reads): List accounts, get balance, view history

These operations have different:
- Performance characteristics
- Validation requirements
- Side effects

## Decision
We adopted **CQRS (Command Query Responsibility Segregation)** at the application layer:

```
Application Layer
├── commands/          # Write operations
│   ├── create_account.rs
│   ├── execute_check_in.rs
│   └── ...
├── queries/           # Read operations
│   ├── get_accounts.rs
│   ├── get_balance.rs
│   └── ...
└── dtos/              # Data Transfer Objects
    ├── account_dto.rs
    └── ...
```

### Principles
1. Commands modify state, return minimal data (success/failure)
2. Queries read state, return rich DTOs
3. Commands may emit domain events
4. Queries are side-effect free

## Consequences

### Positive
- Clear separation between reads and writes
- Easier to optimize queries independently
- Commands can be validated and processed asynchronously
- Natural fit for event-sourcing (future enhancement)

### Negative
- More code organization overhead
- Need to maintain separate command/query handlers
- Can lead to code duplication if not careful

### Current Implementation Status

**Implemented**:
- DTOs for data transfer
- Command-like operations in presentation/commands.rs
- Application services (CheckInExecutor, Scheduler)

**Not Yet Implemented**:
- Separate command/query handler modules
- Domain events publication/subscription
- Event sourcing for audit trail

### Example Structure (Target)

```rust
// Command
pub struct CreateAccountCommand {
    pub name: String,
    pub provider_id: String,
    pub cookies: HashMap<String, String>,
}

// Command Handler
impl CommandHandler<CreateAccountCommand> for CreateAccountHandler {
    async fn handle(&self, cmd: CreateAccountCommand) -> Result<AccountId, Error> {
        // Validate, create aggregate, save, emit events
    }
}

// Query
pub struct GetAccountsQuery {
    pub enabled_only: bool,
}

// Query Handler
impl QueryHandler<GetAccountsQuery> for GetAccountsHandler {
    async fn handle(&self, query: GetAccountsQuery) -> Result<Vec<AccountDto>, Error> {
        // Read from repository, map to DTOs
    }
}
```

### Current Compromise
Business logic is currently in `presentation/commands.rs` rather than separate handlers. This works but should be refactored for better maintainability.

## Alternatives Considered
- **Simple CRUD**: Easier but mixes read/write concerns
- **Event Sourcing**: Too complex for current needs
- **Full CQRS with separate read models**: Overkill for desktop app

## Related
- [Architecture Overview](../architecture_overview.md)
- [ADR-003: DDD Architecture](./003-ddd-architecture.md)
