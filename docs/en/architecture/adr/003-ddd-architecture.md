# ADR-003: DDD Architecture

## Status
Accepted

## Context
NeuraDock has complex business logic around:
- Account management with credentials
- Check-in execution with retry logic
- Balance tracking and caching
- Auto-scheduling with time-based triggers

We need an architecture that:
- Separates business logic from infrastructure concerns
- Makes the codebase maintainable as it grows
- Allows testing business logic in isolation
- Provides clear boundaries between concerns

## Decision
We adopted **Domain-Driven Design (DDD)** with a 4-layer architecture:

```
┌─────────────────────────────────────┐
│  Presentation Layer (Tauri IPC)     │
└─────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────┐
│  Application Layer (Orchestration)  │
└─────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────┐
│  Domain Layer (Business Logic)      │
└─────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────┐
│  Infrastructure Layer (External)    │
└─────────────────────────────────────┘
```

### Layer Rules
1. **Domain Layer** has NO dependencies on other layers
2. Dependencies flow inward (Presentation → Application → Domain ← Infrastructure)
3. Domain defines repository traits, Infrastructure implements them
4. DTOs used for cross-layer communication

## Consequences

### Positive
- Business logic isolated in domain layer
- Easy to test domain logic without mocking infrastructure
- Clear separation of concerns
- Framework-agnostic domain (could port to different UI)
- Repository pattern enables swapping storage implementations

### Negative
- More boilerplate code (DTOs, repository traits)
- Learning curve for developers new to DDD
- Can be over-engineering for simple features
- Need discipline to maintain layer boundaries

### Key Aggregates
- **Account**: Root entity managing account state, credentials, balance cache
- **CheckInJob**: Represents a check-in execution with status state machine

### Implementation Examples

```rust
// Domain layer - pure business logic
impl Account {
    pub fn record_check_in(&mut self) {
        self.last_check_in_at = Some(Utc::now());
    }

    pub fn is_balance_stale(&self, max_age_hours: i64) -> bool {
        // Pure business logic, no infrastructure dependencies
    }
}

// Domain layer - repository trait
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError>;
    async fn save(&self, account: &Account) -> Result<(), DomainError>;
}

// Infrastructure layer - implementation
impl AccountRepository for SqliteAccountRepository {
    async fn find_by_id(&self, id: &AccountId) -> Result<Option<Account>, DomainError> {
        // SQLite-specific implementation
    }
}
```

### Alternatives Considered
- **Clean Architecture**: Similar but more layers (use cases, entities, etc.)
- **Hexagonal Architecture**: Ports and adapters - DDD is similar but more domain-focused
- **Simple MVC**: Would work but harder to maintain as complexity grows

## Related
- [Architecture Overview](../architecture_overview.md)
- [ADR-004: CQRS Pattern](./004-cqrs-pattern.md)
