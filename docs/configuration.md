# 配置指南

## 应用设置

NeuraDock 设置可通过侧边栏的**设置**页面访问。

### 常规设置

| 设置 | 描述 | 默认值 |
|-----|------|-------|
| 语言 | 界面语言（English/中文） | 系统语言 |
| 主题 | 浅色/深色/跟随系统 | 跟随系统 |
| 余额缓存时间 | 余额被视为过期的小时数 | 1 小时 |

### 自动签到设置

为每个账号配置自动签到行为：

| 设置 | 描述 |
|-----|------|
| 启用自动签到 | 为此账号开启/关闭自动签到 |
| 签到时间 | 执行签到的时间（24小时制） |

## 账号配置

### 添加账号

1. 导航到**账号**页面
2. 点击**添加账号**按钮
3. 填写必填字段：
   - **名称**：账号的显示名称
   - **服务商**：服务提供商（AnyRouter、AgentRouter）
   - **Cookies**：认证 cookies（JSON 格式）
   - **API 用户**：API 用户标识符

### Cookie 格式

Cookies 应以 JSON 格式提供：

```json
{
  "session": "your_session_token",
  "cf_clearance": "cloudflare_clearance_cookie"
}
```

**如何获取 cookies：**
1. 登录到你的服务商网站
2. 打开浏览器开发者工具（F12）
3. 转到 Application → Cookies
4. 复制所需的 cookie 值

### 批量导入/导出

**从 JSON 导入：**
```json
[
  {
    "name": "account@example.com",
    "provider": "anyrouter",
    "cookies": {
      "session": "token_value"
    },
    "api_user": "user_id"
  }
]
```

**导出账号：**
- 导航到账号页面
- 选择要导出的账号
- 点击导出按钮
- 选择是否包含凭证

## 数据库位置

NeuraDock 将数据存储在 SQLite 数据库中：

| 平台 | 位置 |
|-----|------|
| macOS | `~/Library/Application Support/com.neuradock.app/neuradock.db` |
| Windows | `%APPDATA%\com.neuradock.app\neuradock.db` |
| Linux | `~/.local/share/com.neuradock.app/neuradock.db` |

**开发数据库**：`neuradock_dev.db`（同目录）

## 服务商配置

内置服务商在应用中配置：

| 服务商 | 基础 URL | WAF 绕过 |
|-------|----------|---------|
| AnyRouter | `https://api.anyrouter.com` | 需要 |
| AgentRouter | `https://api.agentrouter.com` | 不需要 |

## 环境变量

目前，NeuraDock 不使用环境变量。所有配置存储在应用设置和数据库中。

## 高级配置

### 自定义浏览器路径（WAF 绕过）

NeuraDock 自动检测 Chromium 浏览器。支持的浏览器：
- Google Chrome
- Microsoft Edge
- Brave 浏览器
- Chromium

检测路径：
- **macOS**: `/Applications/Google Chrome.app`, `/Applications/Brave Browser.app`
- **Windows**: 注册表和 Program Files
- **Linux**: `/usr/bin/google-chrome`, `/usr/bin/chromium`
