# Changelog

All notable changes to NeuraDock will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

_No entries yet_

---

## [0.5.0] - 2025-12-21

### Added

#### Core Features
- **Bulk credential update**: New BatchUpdateDialog with JSON import, bulk modification, and "create if missing" strategy for syncing large account batches
- **Advanced check-in analytics**: Monthly overview, trend charts, streak reminders, and calendar deep-linking to quickly spot missed check-ins and quota spikes
- **Balance overview refactor**: Provider-level statistics, global totals, and historical consumption comparison on Accounts and Dashboard pages, aligned with new `total_quota` semantics
- **Configurable check-in interval**: Support custom check-in intervals per account (default 24h) to avoid rate limiting
- **Proxy configuration**: In-app proxy settings with HTTP/HTTPS/SOCKS5 support
- **Provider node management**: Add, edit, and delete custom API nodes for flexible relay configurations

#### Architecture Improvements
- **Type-safe IPC**: Introduced tauri-specta for auto-generated TypeScript bindings, providing complete type safety
- **DDD layer decoupling**: Fully decoupled application layer from infrastructure, removed sqlx dependency from upper layers, improving maintainability
- **Modular refactoring**: Backend code split into focused sub-modules by responsibility, enhancing code organization and readability

#### Performance Optimizations
- **Startup performance**: Optimized application initialization, reducing startup time by ~40%
- **Vendor chunks**: Split vendor chunks in frontend build, improving load speed and cache efficiency
- **Scheduler optimization**: Scheduler only loads enabled accounts, reducing memory footprint and scheduling overhead
- **Window persistence**: Window size and position persistence for better UX

#### User Experience
- **Window dragging**: Support dragging from page whitespace for improved convenience
- **I18n enhancements**: Added more UI text translations in Chinese and English
- **Error handling**: Improved error messages, avoided panics, provided more user-friendly feedback

### Changed

#### Breaking Changes
- **Unified quota fields**:
  - Database, DTOs, frontend, and notifications switched to `total_quota` (total quota) semantics
  - Deprecated legacy `total_income = quota + used_quota` definition
  - Upgrading from 0.1.0 will auto-migrate balance data, but recommend verifying consistency

#### Notification System
- **Notification upgrades**: Feishu messages now include yesterday/today/delta comparison blocks
- **Smart fallback**: Automatically falls back to most recent record when history is missing, ensuring notification availability

#### Dependency Updates
- **Frontend dependencies**:
  - React Query → TanStack Query v5
  - Vite upgraded to 6.x
  - React Router upgraded to latest
  - Improved performance and DX

#### Development Tools
- **Makefile simplification**: Kept common commands, removed redundant targets, clearer output
- **CI/CD optimization**: Improved GitHub Actions workflows with multi-platform parallel builds
- **Test enhancements**: Added and fixed backend test cases, improving code quality

### Fixed

#### Feature Fixes
- Fixed Feishu notifications showing only today's balance, restored full three-part comparison format
- Fixed batch check-in results missing account context information
- Fixed unreliable window dragging in certain scenarios
- Fixed Provider node URL validation logic errors

#### Backend Fixes
- Fixed potential panics in query and i18n modules
- Removed unsafe transaction module, replaced with safer implementation
- Fixed inconsistent check-in interval expectations in test cases

#### Frontend Fixes
- Removed duplicate PageContainer component, unified layout components
- Fixed Tauri invoke parameter alignment issues
- Fixed theme switching and UI display issues

### Security

#### Improvements
- Improved error handling to prevent sensitive information leakage in logs
- Continued progress on credential encryption (planned for 0.6.0)
- Minimized log output to reduce potential information leakage risks

### Performance

#### Measurements & Improvements
- Added performance monitoring and benchmarking framework
- Startup time optimization (reduced by ~40%)
- Frontend initial load time optimization (reduced by ~30%)
- Scheduler memory footprint optimization (only loads enabled accounts)

### Documentation

#### Documentation Updates
- **Release guide**: Added complete version release workflow documentation
- **Development docs**: Updated development guide covering 0.5.0 architecture changes
- **README rewrite**: Comprehensive bilingual README rewrite reflecting latest features and architecture
- **CHANGELOG**: Detailed change records for users and developers to understand version differences

### Known Issues

- Credentials still stored in plaintext (encryption planned for 0.6.0)
- Some notification channels (Email, Telegram) not yet implemented
- Some advanced features (model usage statistics) still in development

---

## [0.1.0] - 2025-11-11

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
| 0.5.0 | 2025-12-21 | Unified quota semantics & advanced streak analytics |
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
