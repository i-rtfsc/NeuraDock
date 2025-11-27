# ADR-001: 使用 React 作为前端

## 状态
已采纳

## 背景
NeuraDock 需要一个前端框架来构建桌面应用程序 UI。应用需要：
- 账号和签到操作的复杂状态管理
- 来自后端事件的实时更新
- 账号配置的表单处理
- 国际化支持
- 良好的 TypeScript 开发体验

## 决策
我们选择了 **React 18** 及以下生态系统：
- **TypeScript 5**: 类型安全
- **TanStack Query v5**: 服务器状态管理
- **Zustand**: 客户端状态管理
- **React Router v7**: 导航
- **Tailwind CSS + Radix UI**: 样式和组件
- **react-i18next**: 国际化

## 后果

### 优点
- 庞大的生态系统和社区支持
- 优秀的 TypeScript 集成
- TanStack Query 处理缓存、重新获取和同步
- Radix UI 提供可访问的、无样式组件
- 通过 tauri-specta 轻松与 Tauri 集成

### 缺点
- 需要仔细管理重新渲染
- 打包体积比 Solid 或 Svelte 等替代方案大
- 需要手动使用 useCallback/useMemo 优化

### 考虑过的替代方案
- **Vue 3**: 好的选择，但团队对 React 更熟悉
- **Solid**: 更小的打包体积但生态系统较小
- **Svelte**: 基于编译器的方法，决策时 TypeScript 成熟度较低

## 相关
- [架构概览](../architecture_overview.md)
- [ADR-003: DDD 架构](./003-ddd-architecture.md)
