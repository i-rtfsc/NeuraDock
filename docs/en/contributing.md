# Contributing Guide

Thank you for your interest in contributing to NeuraDock! This guide will help you get started.

## Development Environment Setup

### Prerequisites

- **Node.js**: >= 20.0.0
- **Rust**: >= 1.70.0 (install via [rustup](https://rustup.rs/))
- **npm**: Latest version
- **Git**: For version control
- **IDE**: VS Code recommended with Rust Analyzer and ESLint extensions

### Clone and Setup

```bash
# Clone the repository
git clone https://github.com/i-rtfsc/NeuraDock.git
cd NeuraDock

# Install dependencies
make install

# Start development server
make dev
```

## Project Structure

```
neuradock/
├── apps/
│   └── desktop/              # Tauri desktop application
│       ├── src/              # React frontend
│       │   ├── components/   # UI components
│       │   ├── pages/        # Page components
│       │   ├── hooks/        # Custom React hooks
│       │   ├── lib/          # Utilities and Tauri bindings
│       │   └── i18n/         # Internationalization
│       └── src-tauri/        # Rust backend
│           └── src/
│               ├── domain/           # Domain layer (DDD)
│               ├── application/      # Application layer (CQRS)
│               ├── infrastructure/   # Infrastructure layer
│               └── presentation/     # Presentation layer (Tauri IPC)
├── docs/                     # Documentation
└── migrations/               # Database migrations
```

## Development Commands

### Quick Start

```bash
# First time - install dependencies
make install

# Start development server (with hot reload)
make dev
```

### Complete Command List

#### 📦 Installation and Dependencies

```bash
make install            # Install dependencies
make doctor             # Check development environment
```

#### 🚀 Development Mode

```bash
make dev                # Start development mode (RUST_LOG=info)
make dev-debug          # Start development mode (RUST_LOG=debug - verbose logs)
make dev-trace          # Start development mode (RUST_LOG=trace - performance tracing)
make dev-warn           # Start development mode (RUST_LOG=warn - warnings only)
make kill               # Kill all running processes
```

#### 📦 Build Commands

```bash
make build              # Build Release version (no packaging)
make package            # Build and package Release version (generate installers)
make build-frontend     # Build frontend only
make build-backend      # Build backend only
make bindings           # Generate TypeScript bindings
```

#### 🧪 Test Commands

```bash
make test               # Run all tests
make test-backend       # Run backend tests
make test-frontend      # Run frontend tests
```

#### 🧹 Clean Commands

```bash
make clean              # Clean all build artifacts
make purge              # Deep clean (artifacts + deps + db)
```

#### ✅ Code Quality

```bash
make check              # Check code format (rustfmt + clippy)
make fix                # Auto-fix code format
```

#### 🔧 Tools and Information

```bash
make migrate            # Run database migrations
make help               # Show help for all commands
```

### Common Command Combinations

```bash
# Restart development server
make kill dev

# Clean and rebuild
make clean build

	# Complete release workflow
	make purge
	make install
	make package
```

## Code Style Guidelines

### Rust

- Follow the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- Use `snake_case` for functions and variables
- Use `PascalCase` for types, structs, and enums
- Prefer `Result<T, DomainError>` for domain operations
- Use `anyhow::Result<T>` for application/infrastructure operations
- Run `cargo fmt` before committing

### TypeScript/React

- Strict mode is enabled
- Use `camelCase` for functions and variables
- Use `PascalCase` for components and types
- Prefer `const` over `let`
- Use functional components with hooks
- Import from `@/` alias for src directory

## Architecture Guidelines

NeuraDock follows **DDD (Domain-Driven Design)** with **CQRS** pattern:

1. **Domain Layer** (`src-tauri/src/domain/`)
   - Contains core business logic
   - NO dependencies on other layers
   - Define aggregates, entities, value objects
   - Define repository traits (interfaces)

2. **Application Layer** (`src-tauri/src/application/`)
   - Orchestrates domain operations
   - Command/Query handlers
   - DTOs for data transfer
   - Application services

3. **Infrastructure Layer** (`src-tauri/src/infrastructure/`)
   - Implements repository traits
   - Database persistence (SQLite + sqlx)
   - HTTP client, browser automation
   - External service integrations

4. **Presentation Layer** (`src-tauri/src/presentation/`)
   - Tauri commands (IPC endpoints)
   - Event emissions to frontend
   - State management

## Adding New Features

Follow this checklist:

1. **Domain Layer First**
   - Add/modify aggregates in `domain/`
   - Define repository traits if needed
   - Create value objects for validated data

2. **Infrastructure Implementation**
   - Implement repository traits in `infrastructure/persistence/`
   - Add database migrations if needed
   - Implement external integrations

3. **Application Layer Services**
   - Create command/query handlers
   - Define DTOs in `application/dtos/`
   - Add services for complex workflows

4. **Presentation Layer**
   - Add Tauri commands with `#[tauri::command]` and `#[specta::specta]` macros
   - Register in `main.rs` via `collect_commands![]`
   - Run dev server to regenerate TypeScript bindings

5. **Frontend Implementation**
   - Import from `@/lib/tauri`
   - Create React components
   - Use TanStack Query for data fetching

## Pull Request Process

1. **Fork** the repository

2. **Create a branch** for your feature:
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make your changes** following the guidelines above

4. **Test your changes**:
   ```bash
   cargo test
   npm run typecheck
   ```

5. **Commit with clear messages**:
   ```bash
   git commit -m "feat: add batch account update feature"
   ```

   Follow [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat:` new feature
   - `fix:` bug fix
   - `docs:` documentation
   - `refactor:` code refactoring
   - `test:` adding tests
   - `chore:` maintenance

6. **Push and create PR**:
   ```bash
   git push origin feature/your-feature-name
   ```

7. **PR Review**: Wait for review and address feedback

## Testing

- **Rust unit tests**: Located in `#[cfg(test)]` modules or `*_test.rs` files
- **Use `mockall`** for repository mocking
- **Domain logic** should have comprehensive tests
- **Integration tests** for critical paths

## Documentation

- Update relevant docs when adding features
- Add JSDoc comments for exported TypeScript functions
- Document Rust public APIs with `///` comments
- Update CHANGELOG.md for user-facing changes

## Getting Help

- **GitHub Issues**: Report bugs or request features
- **Discussions**: Ask questions or discuss ideas
