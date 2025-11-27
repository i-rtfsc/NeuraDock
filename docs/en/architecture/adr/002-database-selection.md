# ADR-002: Database Selection

## Status
Accepted

## Context
NeuraDock needs a database for storing:
- Account information and credentials
- Check-in job history
- Balance history over time
- Application settings

Requirements:
- Embedded (no external server)
- Cross-platform (macOS, Windows, Linux)
- Good Rust support
- Reasonable query capabilities
- File-based for easy backup

## Decision
We chose **SQLite** with **sqlx** crate for:
- Database engine: SQLite 3
- Rust driver: sqlx with compile-time checked queries
- Migrations: sqlx::migrate!() macro (embedded in binary)

## Consequences

### Positive
- Zero configuration, file-based storage
- Excellent cross-platform support
- sqlx provides compile-time query verification
- Automatic migrations on startup
- Well-understood technology

### Negative
- Limited concurrent write performance (acceptable for desktop app)
- No built-in encryption (must implement separately)
- Migrations cannot be rolled back easily in production
- Foreign key constraints not enforced by default (need PRAGMA)

### Implementation Notes
```rust
// Enable foreign keys
sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;

// Migrations embedded at compile time
sqlx::migrate!("./migrations").run(&pool).await?;
```

### Security Concern
SQLite stores data unencrypted. Credentials should be encrypted at application level before storage. See [Security Architecture](../architecture_overview.md#security-architecture).

### Alternatives Considered
- **PostgreSQL/MySQL**: Overkill for desktop app, requires external server
- **RocksDB**: Key-value only, no SQL queries
- **Sled**: Pure Rust but less mature
- **Duckdb**: Good for analytics but less suitable for OLTP

## Related
- [Architecture Overview](../architecture_overview.md)
- [Database Schema](../architecture_overview.md#database-schema)
