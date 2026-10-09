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
  "session": "your_session_token"
}
```

> **注意**：只需提供 `session` cookie，其他 cookies 会在运行时自动获取。

**如何获取 cookies：**
1. 登录到你的服务商网站
2. 打开浏览器开发者工具（F12）
3. 转到 Application → Cookies
4. 复制 `session` cookie 的值

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

## Token 配置

### AI 工具配置

NeuraDock 可以为 AI 开发工具配置 Token，支持：

| 工具 | 配置文件 | 说明 |
|-----|---------|------|
| **Claude Code** | `~/.claude/settings.json` | Anthropic Claude CLI 工具 |
| **Codex** | `~/.codex/config.toml` 或 `~/.codex/profiles.d/<配置名>.config.toml` | 支持默认配置和命名配置 |

### 配置 Token

1. 导航到 **Token 管理** 页面
2. 选择账号（AnyRouter 或 AgentRouter）
3. 点击 Token 旁的 **配置** 按钮
4. 选择 AI 工具和节点
5. 点击 **应用配置**

### Codex 多账号和多模型配置

在 **Agent 配置 → Codex → 添加配置** 中选择中转站、账号、Key 和节点，使用独立配置文件，勾选多个模型并指定一个默认模型。

配置名采用“中转站简称－账号邮箱域名前缀”，例如 AnyRouter 的 `user@qq.com` 对应 `any-qq`，AgentRouter 的 Gmail 账号对应 `agent-gmail`。同名配置自动添加数字后缀；账号名称不含邮箱时，可手动填写域名前缀。编辑已有配置时保留其名称。

每份配置生成 `~/.codex/profiles.d/any-qq.config.toml` 和 `any-qq.models.json`，并在 `~/.codex/any-qq.config.toml` 建立软链接。复制启动命令 `codex -p any-qq` 即可使用。所有中转站的 provider ID 固定为 `3rd`，使用同一个 Codex 主目录。

Claude Code 也采用相同的账号命名：保存到 `~/.claude/profiles.d/any-qq.json`，通过 `claude --settings ~/.claude/profiles.d/any-qq.json` 启动。旧版 `settings-*.json` 继续识别，并保留原文件路径。

模型只能从服务端提供的列表选择；列表为空或获取失败时使用“刷新模型列表”，不支持手动添加名称。独立 Key 从其配置的模型接口读取列表。

模型列表还会按 Agent 协议过滤：Codex 配置不提供 Claude 模型，后台也会拒绝保存此类配置。通过配置列表的“更多 → 删除配置”可移除单个命名配置及其模型目录、软链接和绑定记录；默认配置使用“清除接入配置”，保留其他设置。

AgentRouter 提供 `https://agentrouter.org` 和 `https://ps.air-outer.com` 两个请求节点。AnyRouter 保留上海节点，rainapp 节点已停用，启动新版应用时会清理已保存的 rainapp 节点记录。

DeepSeek V4 Flash、Pro 使用随应用提供的模型模板；其他模型自动补齐目录所需字段。上下文和推理能力由各自模型条目配置。修改模型目录后，需要重新启动对应的 Codex 会话。

### 自定义节点

如果你有自定义 API 端点，可以添加自定义节点：

1. 点击 **管理节点** 按钮
2. 选择服务提供商
3. 输入节点名称和 URL
4. 点击 **添加**

**示例节点配置：**
```
名称: 我的私有节点
URL: https://api.example.com
```

### 清除配置

要清除 NeuraDock 管理的配置：

1. 点击 **清除配置** 按钮
2. 选择要清除的工具（Claude Code 或 Codex）
3. 确认操作



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
