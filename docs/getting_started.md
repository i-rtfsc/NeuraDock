# 快速入门

## 什么是 NeuraDock？

NeuraDock 是一个现代化的自动签到管理系统，基于 **Tauri 2 + Rust + React** 构建。它帮助你管理多个服务商账号、自动执行每日签到、追踪余额历史。

## 核心功能

- **多账号管理**：管理不同服务商的账号（AnyRouter、AgentRouter 等）
- **自动签到**：在指定时间自动执行每日签到
- **余额追踪**：监控配额使用情况和余额历史
- **WAF 绕过**：使用浏览器自动化自动绕过 Cloudflare 保护
- **会话缓存**：智能会话管理减少浏览器自动化开销
- **跨平台**：支持 macOS、Windows 和 Linux

## 快速开始

1. **下载最新版本** 从 [Releases](https://github.com/neuradock/neuradock/releases) 页面

2. **安装应用**：
   - **macOS**: 打开 `.dmg` 文件，将 NeuraDock 拖入 Applications
   - **Windows**: 运行 `.msi` 安装程序
   - **Linux**: 运行 `.AppImage` 文件

3. **启动 NeuraDock** 并添加你的第一个账号

4. **配置自动签到**（可选）以自动执行每日签到

## 系统要求

| 平台 | 最低版本 |
|------|---------|
| macOS | 10.15 (Catalina) 或更高 |
| Windows | Windows 10 或更高 |
| Linux | Ubuntu 20.04 或同等版本 |

**额外要求**：
- 需要安装 Chromium 内核浏览器（Chrome、Edge、Brave）以支持 WAF 绕过功能

## 下一步

- [安装指南](./installation.md) - 详细安装说明
- [配置指南](./configuration.md) - 配置账号和设置
- [用户指南](./user_guide/README.md) - 完整使用文档
