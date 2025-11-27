<div align="center">

# NeuraDock

**Modern Automatic Check-in Management System**

[中文](README.md) | English

<!-- Core Tech Stack -->
[![Tauri](https://img.shields.io/badge/Tauri-2.1-24C8D8?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-1.70+-DEA584?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=white)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Node.js](https://img.shields.io/badge/Node.js-20+-339933?style=flat-square&logo=node.js&logoColor=white)](https://nodejs.org/)

<!-- Project Info -->
[![Version](https://img.shields.io/badge/version-0.1.0-brightgreen?style=flat-square)](https://github.com/neuradock/neuradock/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20|%20Windows%20|%20Linux-lightgrey?style=flat-square)](https://github.com/neuradock/neuradock/releases)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen?style=flat-square)](docs/en/contributing.md)

<!-- Frontend Tech Stack -->
[![Vite](https://img.shields.io/badge/Vite-6-646CFF?style=flat-square&logo=vite&logoColor=white)](https://vitejs.dev/)
[![Tailwind CSS](https://img.shields.io/badge/Tailwind%20CSS-3-06B6D4?style=flat-square&logo=tailwindcss&logoColor=white)](https://tailwindcss.com/)
[![SQLite](https://img.shields.io/badge/SQLite-3-003B57?style=flat-square&logo=sqlite&logoColor=white)](https://www.sqlite.org/)

<!-- Code Style -->
[![Code style: rustfmt](https://img.shields.io/badge/code%20style-rustfmt-DEA584?style=flat-square)](https://github.com/rust-lang/rustfmt)
[![Code style: prettier](https://img.shields.io/badge/code%20style-prettier-ff69b4?style=flat-square)](https://prettier.io/)

</div>

---

## 📖 Overview

NeuraDock is a modern desktop application built with **Tauri 2 + Rust + React**, using **DDD (Domain-Driven Design) + CQRS** architecture. It supports multi-provider account management, automatic check-ins, balance tracking, and more.

### ✨ Key Features

- 🔐 **Multi-Account Management** - Unified management for multiple service provider accounts
- ⏰ **Auto Check-in** - Configurable daily automatic check-in scheduling
- 📊 **Balance Tracking** - Quota usage monitoring and history records
- 🛡️ **WAF Bypass** - Browser automation to bypass Cloudflare protection
- 💾 **Session Caching** - Intelligent session management to reduce browser automation overhead
- 🌐 **Cross-Platform** - Supports macOS, Windows, and Linux
- 🌍 **Internationalization** - Supports Chinese and English interfaces

---

## 🛠️ Tech Stack

<table>
<tr>
<td width="50%">

### Backend (Rust)

| Technology | Description |
|------------|-------------|
| **Tauri 2.1** | Desktop app framework |
| **DDD + CQRS** | Architecture pattern |
| **SQLite + sqlx** | Database |
| **tauri-specta** | Type-safe IPC |
| **reqwest** | HTTP client |
| **chromiumoxide** | Browser automation |

</td>
<td width="50%">

### Frontend (React)

| Technology | Description |
|------------|-------------|
| **React 18** | UI framework |
| **TypeScript 5** | Type safety |
| **Vite 6** | Build tool |
| **TanStack Query v5** | Server state |
| **Tailwind CSS** | Styling |
| **Radix UI** | Accessible components |

</td>
</tr>
</table>

---

## 📦 Quick Start

### Requirements

| Dependency | Version |
|------------|---------|
| Node.js | >= 20.0.0 |
| Rust | >= 1.70.0 |
| System | macOS 10.15+ / Windows 10+ / Linux (Ubuntu 20.04+) |

### Installation

```bash
# Clone the repository
git clone https://github.com/neuradock/neuradock.git
cd neuradock

# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build
```

### Build Outputs

| Platform | Path |
|----------|------|
| macOS | `apps/desktop/src-tauri/target/release/bundle/dmg/` |
| Windows | `apps/desktop/src-tauri/target/release/bundle/msi/` |
| Linux | `apps/desktop/src-tauri/target/release/bundle/appimage/` |

---

## 🏗️ Project Structure

```
neuradock/
├── apps/
│   └── desktop/                    # Tauri desktop application
│       ├── src/                    # React frontend
│       │   ├── components/         # UI components
│       │   ├── pages/              # Page components
│       │   ├── hooks/              # Custom hooks
│       │   └── lib/                # Utilities
│       └── src-tauri/              # Rust backend
│           └── src/
│               ├── domain/         # Domain layer (DDD)
│               ├── application/    # Application layer (CQRS)
│               ├── infrastructure/ # Infrastructure layer
│               └── presentation/   # Presentation layer
├── docs/                           # Chinese documentation
│   └── en/                         # English documentation
└── migrations/                     # Database migrations
```

---

## 🏛️ Architecture

NeuraDock follows a **DDD 4-Layer Architecture**:

```
┌─────────────────────────────────────┐
│    Presentation Layer (Tauri IPC)   │  ← Tauri commands & events
├─────────────────────────────────────┤
│    Application Layer (CQRS)         │  ← Command/Query handlers
├─────────────────────────────────────┤
│    Domain Layer (Core)              │  ← Business logic (no deps)
├─────────────────────────────────────┤
│    Infrastructure Layer             │  ← SQLite, HTTP, Browser
└─────────────────────────────────────┘
```

### Key Design Decisions

- 📝 **Type-Safe IPC** - Auto-generated TypeScript bindings via tauri-specta
- 🔀 **CQRS Separation** - Commands modify state, queries read state
- 📡 **Event-Driven** - Decoupling through domain events
- 🗄️ **Repository Pattern** - Abstracted data access layer

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [Getting Started](docs/en/getting_started.md) | Start using NeuraDock |
| [Installation](docs/en/installation.md) | Detailed installation guide |
| [Configuration](docs/en/configuration.md) | Configure accounts and settings |
| [User Guide](docs/en/user_guide/README.md) | Complete usage documentation |
| [Architecture](docs/en/architecture/architecture_overview.md) | System architecture design |
| [API Reference](docs/en/api/api_reference.md) | Tauri IPC commands |
| [Contributing](docs/en/contributing.md) | How to contribute |

---

## 🗺️ Roadmap

### Phase 1: Tauri Desktop App ✅ In Progress

- [x] DDD domain layer architecture
- [x] SQLite database layer
- [x] tauri-specta type-safe IPC
- [x] Account CRUD operations
- [x] JSON import/export
- [ ] Check-in executor (HTTP + WAF bypass)
- [ ] Check-in history and statistics
- [ ] Notification system

### Phase 2: VSCode Extension 🔮 Future

- [ ] Extract shared core to `packages/core`
- [ ] Support WASM compilation
- [ ] Implement VSCode Extension

---

## 🤝 Contributing

Issues and Pull Requests are welcome!

Please read the [Contributing Guide](docs/en/contributing.md) to learn how to participate in project development.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

## 📬 Contact

- 📝 **Issues**: [GitHub Issues](https://github.com/neuradock/neuradock/issues)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/neuradock/neuradock/discussions)

---

<div align="center">

**If this project helps you, please give it a ⭐ Star!**

Made with ❤️ by NeuraDock Team

</div>
