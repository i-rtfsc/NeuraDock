<div align="center">

# NeuraDock

**现代化的自动签到管理系统**

[English](README_EN.md) | 中文

<!-- 核心技术栈 -->
[![Tauri](https://img.shields.io/badge/Tauri-2.1-24C8D8?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-1.70+-DEA584?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=white)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Node.js](https://img.shields.io/badge/Node.js-20+-339933?style=flat-square&logo=node.js&logoColor=white)](https://nodejs.org/)

<!-- 项目信息 -->
[![Version](https://img.shields.io/badge/version-0.1.0-brightgreen?style=flat-square)](https://github.com/neuradock/neuradock/releases)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20|%20Windows%20|%20Linux-lightgrey?style=flat-square)](https://github.com/neuradock/neuradock/releases)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen?style=flat-square)](docs/contributing.md)

<!-- 前端技术栈 -->
[![Vite](https://img.shields.io/badge/Vite-6-646CFF?style=flat-square&logo=vite&logoColor=white)](https://vitejs.dev/)
[![Tailwind CSS](https://img.shields.io/badge/Tailwind%20CSS-3-06B6D4?style=flat-square&logo=tailwindcss&logoColor=white)](https://tailwindcss.com/)
[![SQLite](https://img.shields.io/badge/SQLite-3-003B57?style=flat-square&logo=sqlite&logoColor=white)](https://www.sqlite.org/)

<!-- 代码风格 -->
[![Code style: rustfmt](https://img.shields.io/badge/code%20style-rustfmt-DEA584?style=flat-square)](https://github.com/rust-lang/rustfmt)
[![Code style: prettier](https://img.shields.io/badge/code%20style-prettier-ff69b4?style=flat-square)](https://prettier.io/)

</div>

---

## 📖 项目概述

NeuraDock 是一个基于 **Tauri 2 + Rust + React** 构建的现代桌面应用，采用 **DDD (领域驱动设计) + CQRS** 架构，支持多服务商账号管理、自动签到、余额追踪等功能。

### ✨ 核心特性

- 🔐 **多账号管理** - 支持多个服务商账号的统一管理
- ⏰ **自动签到** - 可配置时间的每日自动签到调度
- 📊 **余额追踪** - 配额使用情况监控与历史记录
- 🛡️ **WAF 绕过** - 使用浏览器自动化绕过 Cloudflare 保护
- 💾 **会话缓存** - 智能会话管理减少浏览器自动化开销
- 🌐 **跨平台** - 支持 macOS、Windows 和 Linux
- 🌍 **国际化** - 支持中文和英文界面

---

## 🛠️ 技术栈

<table>
<tr>
<td width="50%">

### 后端 (Rust)

| 技术 | 说明 |
|------|------|
| **Tauri 2.1** | 桌面应用框架 |
| **DDD + CQRS** | 架构模式 |
| **SQLite + sqlx** | 数据库 |
| **tauri-specta** | 类型安全 IPC |
| **reqwest** | HTTP 客户端 |
| **chromiumoxide** | 浏览器自动化 |

</td>
<td width="50%">

### 前端 (React)

| 技术 | 说明 |
|------|------|
| **React 18** | UI 框架 |
| **TypeScript 5** | 类型安全 |
| **Vite 6** | 构建工具 |
| **TanStack Query v5** | 服务器状态 |
| **Tailwind CSS** | 样式框架 |
| **Radix UI** | 无障碍组件 |

</td>
</tr>
</table>

---

## 📦 快速开始

### 环境要求

| 依赖 | 版本要求 |
|------|---------|
| Node.js | >= 20.0.0 |
| Rust | >= 1.70.0 |
| 系统 | macOS 10.15+ / Windows 10+ / Linux (Ubuntu 20.04+) |

### 安装与运行

```bash
# 克隆仓库
git clone https://github.com/neuradock/neuradock.git
cd neuradock

# 安装依赖
npm install

# 启动开发服务器
npm run dev

# 构建生产版本
npm run build
```

### 构建输出

| 平台 | 路径 |
|------|------|
| macOS | `apps/desktop/src-tauri/target/release/bundle/dmg/` |
| Windows | `apps/desktop/src-tauri/target/release/bundle/msi/` |
| Linux | `apps/desktop/src-tauri/target/release/bundle/appimage/` |

---

## 🏗️ 项目结构

```
neuradock/
├── apps/
│   └── desktop/                    # Tauri 桌面应用
│       ├── src/                    # React 前端
│       │   ├── components/         # UI 组件
│       │   ├── pages/              # 页面组件
│       │   ├── hooks/              # 自定义 Hooks
│       │   └── lib/                # 工具函数
│       └── src-tauri/              # Rust 后端
│           └── src/
│               ├── domain/         # 领域层 (DDD)
│               ├── application/    # 应用层 (CQRS)
│               ├── infrastructure/ # 基础设施层
│               └── presentation/   # 表示层
├── docs/                           # 中文文档
│   └── en/                         # 英文文档
└── migrations/                     # 数据库迁移
```

---

## 🏛️ 架构设计

NeuraDock 采用 **DDD 四层架构**：

```
┌─────────────────────────────────────┐
│    表示层 (Tauri IPC)                │  ← Tauri 命令和事件
├─────────────────────────────────────┤
│    应用层 (CQRS)                     │  ← 命令/查询处理器
├─────────────────────────────────────┤
│    领域层 (核心)                     │  ← 业务逻辑 (无依赖)
├─────────────────────────────────────┤
│    基础设施层                        │  ← SQLite、HTTP、浏览器
└─────────────────────────────────────┘
```

### 关键设计决策

- 📝 **类型安全 IPC** - 使用 tauri-specta 自动生成 TypeScript 绑定
- 🔀 **CQRS 分离** - 命令修改状态，查询读取状态
- 📡 **事件驱动** - 通过领域事件实现解耦
- 🗄️ **仓储模式** - 抽象数据访问层

---

## 📚 文档

| 文档 | 描述 |
|------|------|
| [快速入门](docs/getting_started.md) | 开始使用 NeuraDock |
| [安装指南](docs/installation.md) | 详细安装说明 |
| [配置指南](docs/configuration.md) | 配置账号和设置 |
| [用户指南](docs/user_guide/README.md) | 完整使用文档 |
| [架构概览](docs/architecture/architecture_overview.md) | 系统架构设计 |
| [API 参考](docs/api/api_reference.md) | Tauri IPC 命令 |
| [贡献指南](docs/contributing.md) | 如何贡献代码 |

---

## 🗺️ 路线图

### Phase 1: Tauri 桌面应用 ✅ 进行中

- [x] DDD 领域层架构
- [x] SQLite 数据库层
- [x] tauri-specta 类型安全 IPC
- [x] 账号 CRUD 操作
- [x] JSON 导入/导出
- [ ] 签到执行器 (HTTP + WAF bypass)
- [ ] 签到历史和统计
- [ ] 通知系统

### Phase 2: VSCode 插件 🔮 未来

- [ ] 提取共享核心到 `packages/core`
- [ ] 支持 WASM 编译
- [ ] 实现 VSCode Extension

---

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

请先阅读 [贡献指南](docs/contributing.md) 了解如何参与项目开发。

---

## 📄 许可证

本项目采用 [Apache License 2.0](LICENSE) 许可证。

---

## 📬 联系方式

- 📝 **Issues**: [GitHub Issues](https://github.com/neuradock/neuradock/issues)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/neuradock/neuradock/discussions)

---

<div align="center">

**如果这个项目对你有帮助，请给它一个 ⭐ Star！**

Made with ❤️ by NeuraDock Team

</div>
