# Changelog

All notable changes to NeuraDock will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Batch update feature for bulk credential updates
- BatchUpdateDialog component for JSON-based batch updates
- Support for "create if not exists" in batch operations

### Changed
- Updated TanStack React Query to v5.90.10
- Updated Vite to v6.4.1
- Updated React Router to v7.9.6

### Fixed
- (Pending) HTTP response body double-read bug
- (Pending) Scheduler panic on invalid time values

### Security
- (Pending) Credential encryption at rest
- (Pending) Remove credentials from API responses

---

## [0.1.0] - 2025-01-21

### Added
- Initial release of NeuraDock
- DDD + CQRS architecture with Tauri 2 + Rust + React
- Multi-account management for service providers
- Manual and batch check-in functionality
- Auto check-in scheduling with configurable time
- Balance tracking with caching strategy
- WAF bypass using browser automation (chromiumoxide)
- Session caching to reduce browser automation overhead
- JSON import/export for accounts
- Internationalization support (English, 简体中文)
- SQLite database with automatic migrations
- Type-safe IPC with tauri-specta

### Supported Providers
- AnyRouter (with WAF bypass)
- AgentRouter

### Known Issues
- Credentials stored unencrypted in database
- Some commands not yet implemented (history, stats)
- Notification system is placeholder only

---

## Version History

| Version | Date | Highlights |
|---------|------|------------|
| 0.1.0 | 2025-01-21 | Initial release |

---

## Upgrade Notes

### Upgrading to 0.2.0 (Future)

When 0.2.0 is released with credential encryption:

1. **Backup your database** before upgrading
2. Existing credentials will be migrated automatically
3. If migration fails, re-import accounts from JSON backup

### Database Migrations

Database migrations run automatically on application startup. No manual intervention required.

**Note**: Migrations cannot be rolled back. Always backup before upgrading.
