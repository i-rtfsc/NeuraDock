# ADR-007: 签到状态模型（证据账本 + 尝试履历 + quota day）

## 状态
已采纳并实施（2026-09）。D-1…D-7 全部落地；实施中与原稿的偏差、以及未决项的收敛见文末「实施记录」。

## 背景

### 产品约束（已确认）
1. **「quota 上涨 = 签到」是有意设计**：部分账号当前无法自动签到，只能网页签到；之后 App 查询余额发现额度上涨，即视为该日已签到。这意味着**推断（inferred）证据是一等公民**，不是降级方案。
2. **`last_check_in` 的现有语义是坏设计**，需要重新设计，并且必须覆盖两种现实：**根本签不了**（只能网页签到 / provider 无签到端点）与**临时签不成**（网络、WAF、provider 抖动）。
3. agentrouter 目前确实无法签到（`sign_in_path = NULL`，`check_in_bugged = 1`），后续单独找方案；本 ADR 只要求模型能容纳它，不求解它。
4. `check_in_interval_hours` 是**遗留问题**，默认 0 不是产品意图 → 允许被替换。
5. 「签到历史 / 统计 / 运行中任务」这条切片**要补齐**，不删除。

### 现状模型
- 证据：`check_in_records(account_id, check_in_date, reward_cents, source, recorded_at)`，PK `(account_id, check_in_date)`（`apps/desktop/src-tauri/crates/neuradock-infrastructure/migrations/20260910000001_create_check_in_records.sql:3-10`）。
- 快照：`balance_history`，每账号每日一行，确定性 ID = `sha256(account_id + "%Y-%m-%d")`（`apps/desktop/src-tauri/crates/neuradock-app/src/application/services/balance_history_service.rs:73-98`）。
- 读模型：`DAILY_SUMMARY_QUERY` 取「balance_history 日 ∪ check_in_records 日」，并以 `<= day` 的最新快照做前向填充（`apps/desktop/src-tauri/crates/neuradock-infrastructure/src/persistence/repositories/balance_history_repo.rs:66-85`）。
- 守卫：`CheckInDomainService::can_check_in` 比较 `now - account.last_check_in()` 与 `interval - 15min`（`apps/desktop/src-tauri/crates/neuradock-domain/src/check_in/domain_service.rs:19-53`）。
- 时间列：`accounts.last_check_in`、`accounts.last_balance_check_at`（`.../migrations/20250121000001_initial_schema.sql:20-23`）。

### 已确认缺陷（本 ADR 的动机）
| # | 缺陷 | 证据 |
|---|---|---|
| D1 | `accounts.last_check_in` **只写不读**；SELECT 用 `MAX(balance_history.recorded_at)` 冒充 | 写 `account_repo/mutations.rs:22-30,60`；读 `account_repo/mod.rs:24,36-40` |
| D2 | 于是「最后签到」=「最后一次余额快照」，与同一 DTO 里的 `last_balance_check_at` 是同一来源（近重复字段） | `account_dto.rs:107,159`；`lib/tauri.ts:1353` 同时存在两字段 |
| D3 | 间隔守卫因此可被**纯余额刷新**推进/绕过；反之「签到成功但余额拉取失败」时守卫不前进 | `balance_service.rs:113-117`；`check_in_executor/mod.rs:122-132` vs `:192-205` |
| D4 | **运行时完全没有额度增量推断** —— 产品约束 1 只在一次性回填迁移里实现，迁移之后再无网页签到可被识别 | 生产代码仅两处写证据：`check_in_executor/mod.rs:128`、`balance_history_service.rs:47-65` |
| D5 | 回填把**首日快照**（`increment IS NULL`）也写成 `legacy_estimate` → `is_checked_in() == true` 的幽灵签到 | 迁移 `WHERE (increment IS NULL OR increment > 0)`；`balance_history/types.rs:207-209` |
| D6 | 自然日 = UTC 日，而前端「今天」用本地时间、补记校验又用 `toISOString()` | `balance_history/types.rs:41`；`CheckInCalendar.tsx:70-73`；`CheckInDayDetailDialog.tsx:38` |
| D7 | 无尝试履历：失败不落库、并发无 single-flight、退避无依据；`interval=0` 时守卫整体失效 | `domain_service.rs:31-34`；`batch.rs:88` 顺序循环；`http/client/check_in.rs:16-43` 无条件重试 3 次 |
| D8 | `check_in_jobs` 表 + `CheckInJob` 聚合 + `CheckInJobRepository`（**无任何 impl**）全死，而 4 个命令返回 `Not implemented yet` | `grep check_in_jobs` 在 `*.rs` 零命中；`presentation/commands/check_in.rs:112-148` |

## 决策

### D-1 分离两个概念：**证据（Evidence）** 与 **尝试（Attempt）**
- 证据 = 业务事实：「这个 quota day 的签到已经发生了」，带**置信度**与**来源**。
- 尝试 = 运维遥测：「我们试过，结果如何」，append-only，可裁剪，是退避、single-flight、历史与统计的数据源。
- 两者不可互相替代：失败必须留痕但不产生证据；观察到额度上涨必须产生证据但不虚构尝试。

### D-2 用 **quota day** 取代 24 小时间隔作为主守卫
- `quota_day = date(observed_at + provider.check_in_day_offset_minutes)`，offset 默认 `+480`（UTC+8，CN provider），可由 provider 配置覆盖；未配置时回退 UTC（＝现状，保证行为不回退）。
- 主守卫问的是一个可判定问题：**「今天的签到是否已经成立？」** 而不是「是否已经过了 24h − 15min」。这直接删掉 `INTERVAL_GRACE` 这类补丁，并修掉「调度退化成隔天」「同一天重复 POST」两类问题。
- 保留一个**纯防御性的最小尝试间隔**（仅约束失败的重复尝试，默认 30min，provider 可配），指数退避 5m → 15m → 60m → 6h，封顶到下一个 quota day。

### D-3 删除 `accounts.last_check_in`，改为三个显式、语义唯一的时间
| 字段 | 含义 | 来源 |
|---|---|---|
| `last_confirmed_at` | 最后一次**被确认**的签到（provider 回执 / 人工补记） | 证据账本中 `confidence ∈ {confirmed, manual}` 的最新 `recorded_at` |
| `last_evidence_at` | 最后一次**观测到**签到（含 inferred） | 证据账本最新 `recorded_at` |
| `last_balance_check_at` | 最后一次余额快照（**语义本来就对，保留**） | `balances.last_checked_at` |

- 前端「最后签到」列改读 `last_confirmed_at`（并可在 tooltip 显示 `last_evidence_at` 与「含推断」），彻底消除 D2 的重复字段。
- 实现取「账本为唯一真相 + accounts 上做反规范化缓存列」，写证据时**同一事务**内更新缓存；另提供 `recalculate_check_in_state` 修复命令（同时成为现成 `UnitOfWork` 的第一个真实使用者，`persistence/unit_of_work.rs` 目前零引用）。

### D-4 运行时额度增量推断（补齐产品约束 1）
在**每一次**余额观察（手动刷新、页面加载、未来的自动化）后执行：

```
on_balance_observed(account, provider, snapshot):
    day  = quota_day(snapshot.observed_at, provider.check_in_day_offset_minutes)
    prev = 最近一个 quota_day < day 的快照          # 前向填充基线
    if prev is None:                 return NoBaseline      # 修 D5：首日不产生证据
    delta = (snapshot.total_quota - snapshot.quota_adjustment)
          - (prev.total_quota     - prev.quota_adjustment)   # 必须扣掉人工校正
    if delta <= 0:                   return NoIncrease      # 不产生、也不删除既有证据
    upsert_evidence(day, reward_cents = round(delta*100),
                    source = 'balance_delta', confidence = 'inferred')
```

- **必须扣除 `quota_adjustment`**（`20260809000001_add_quota_adjustment.sql`）：否则一次人工校正会被认成奖励。
- 误报（充值、赠送）是**产品接受的代价**（约束 1）。缓解手段：`confidence='inferred'` 在 UI 可见（复用现有 `reward_is_estimate`），并预留「撤销该天」入口。
- 幂等由 `(account_id, quota_day)` 唯一键保证。

### D-5 置信度优先级格（只升不降）
```
manual > provider_response > provider_rule > balance_delta > legacy_estimate
```
规则：**同一天内**允许「补金额」与「提置信度」；**永不**移动 `quota_day`、**永不**降低置信度。现有 UPSERT 的 `WHERE` 子句（`balance_history_repo.rs:105-112`）是这套格子的雏形，本 ADR 要求把它写成显式规则表 + 单元测试，并补上 `balance_delta` 一档。

### D-6 streak / 日历 / 趋势以证据为准，并锚定 quota day
- 一天计入连续 ⟺ 该 quota day 存在置信度足够的证据（`confirmed`/`manual` 直接计入；`inferred` 默认计入并在 DTO 暴露 `includes_inferred` 与推断天数）。
- **`current_streak` 锚定今天**：最后证据日 `< today-1` ⇒ 0；`= today-1` ⇒ 保留（今天还有机会）；`= today` ⇒ 正常累加。修掉「尾部 gap 无行时保留过期值」。
- `check_in_rate` 分母改为**当月已过天数**，不是整月天数（`calendar.rs:125-129`）。
- 日历首日的 `daily_consumed` 用「上月最后一天」作基线，修掉恒为 0（`calendar.rs:80-87`）。
- 趋势补零（无数据日给出显式 null 点），`trend` 的 `end_date` 与日桶统一走同一个后端 helper。
- **前端禁止自行 `new Date('YYYY-MM-DD')` 推导日**；DTO 直接给 `quota_day` 字符串 + 时区偏移，`CheckInDayDetailDialog.tsx:38` 的 `toISOString()` 比较改由后端给 `can_confirm`。

### D-7 补齐（约束 5）「历史 / 统计 / 运行中任务」
- **历史**：`check_in_attempts` 为数据源，证据做 join；分页、按账号/结果/日期过滤。现有的 `get_check_in_history`（`presentation/commands/check_in.rs:123`）从 `Not implemented` 变为真实实现，DTO 需要增加 `outcome`、`error_code`、`duration_ms`。
- **统计**：成功率、失败原因直方图、平均耗时、奖励合计、推断占比 + 修正后的 streak；复用同一个 streak 服务，避免第二套算法。
- **运行中任务**：需要**决定**（见「未决问题 U2」）——(i) 复活 `check_in_jobs` 作为持久化 job 注册表，则 `CheckInJobRepository` 得到第一个实现、13 个死测试变成真测试；(ii) 删除该表/聚合，`get_running_jobs` 从调度器内存读模型投影。**不建议继续半复活状态**（这正是 D8 的成因）。
- **`stop_check_in`** 需要协作式取消：每个 job 一个取消令牌；批量循环在账号之间检查（`batch.rs:88` 目前是裸顺序循环），HTTP 层 race 取消。

## 数据模型

```sql
-- M1: 尝试履历（append-only）
CREATE TABLE check_in_attempts (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id    TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    provider_id   TEXT NOT NULL,
    quota_day     TEXT NOT NULL,                 -- YYYY-MM-DD（provider 重置日）
    trigger       TEXT NOT NULL CHECK (trigger IN ('manual','batch','scheduler','observe')),
    started_at    TEXT NOT NULL,                 -- RFC3339 UTC
    finished_at   TEXT,
    outcome       TEXT CHECK (outcome IN
                    ('success','already_confirmed','failed_retryable',
                     'failed_permanent','waf_blocked','skipped_guard','cancelled')),
    error_code    TEXT,                          -- 稳定枚举；严禁写入上游 body
    http_status   INTEGER,
    duration_ms   INTEGER,
    reward_cents  INTEGER,
    source        TEXT
);
CREATE INDEX idx_attempts_account_day  ON check_in_attempts(account_id, quota_day);
CREATE INDEX idx_attempts_account_time ON check_in_attempts(account_id, started_at DESC);
```

```sql
-- M2: 证据账本加置信度与尝试关联
ALTER TABLE check_in_records ADD COLUMN confidence TEXT NOT NULL DEFAULT 'confirmed'
    CHECK (confidence IN ('confirmed','inferred','manual'));
ALTER TABLE check_in_records ADD COLUMN attempt_id INTEGER
    REFERENCES check_in_attempts(id) ON DELETE SET NULL;
-- 显式回填（不依赖列默认值）
UPDATE check_in_records SET confidence = CASE source
    WHEN 'legacy_estimate'   THEN 'inferred'
    WHEN 'manual'            THEN 'manual'
    ELSE 'confirmed' END;
-- 用 recorded_at（完整 UTC 时间戳）+ provider offset 重算 quota_day，
-- 使历史行与新口径一致；legacy 行无法区分充值，保持 inferred。
UPDATE check_in_records SET check_in_date = <recomputed quota_day>;
```
> 写入端必须显式提供 `confidence`；列默认值只是迁移产物。

```sql
-- M3: accounts 反规范化缓存 + provider 配置
ALTER TABLE accounts  ADD COLUMN last_confirmed_at TEXT;
ALTER TABLE accounts  ADD COLUMN last_evidence_at  TEXT;
ALTER TABLE providers ADD COLUMN check_in_day_offset_minutes INTEGER NOT NULL DEFAULT 480;
ALTER TABLE providers ADD COLUMN check_in_mode TEXT NOT NULL DEFAULT 'api'
    CHECK (check_in_mode IN ('api','page_visit','observe_only','disabled'));
-- 切换读路径完成后（M5）：
ALTER TABLE accounts DROP COLUMN last_check_in;   -- 需 SQLite ≥ 3.35，需确认 bundled 版本
```

### `AccountDto` 目标形态（前端契约）
```
last_confirmed_at: string | null      // 表格「最后签到」
last_evidence_at:  string | null      // 含推断
last_balance_check_at: string | null  // 语义不变
today_check_in_state: 'done_confirmed' | 'done_inferred' | 'due'
                    | 'in_flight' | 'backoff'
```
`today_check_in_state` 由后端单点计算（D-2 的状态机），前端不再用时间戳自行判断按钮状态。

### 状态机
```
check_in_state(account, provider, evidence[day], attempts[day], now) -> State
  day = quota_day(now, provider.offset)
  if evidence[day].confidence in {confirmed, manual, inferred}: return Done
  if attempts[day] with (finished_at IS NULL and started_at > now - lease): return InFlight
  if backoff_until(attempts, day) > now: return Backoff(until)
  return Due
```
- `Done` 同时解决「重复 POST」与「已签到却仍显示失败」。
- `InFlight` 是 single-flight 租约，修掉并发重复签发（`tests.rs:145-148` 目前只验证了账本去重，没验证只发一次请求）。
- `Backoff` 承接「临时签不成」：当天内重试，但绝不重发已成功的尝试。

## 不变式（每条对应一个测试）
1. 纯余额刷新**不**改变 `last_confirmed_at`；只有产生推断证据时才改变 `last_evidence_at`。
2. 同一 `(account_id, quota_day)`：证据至多 1 行，尝试可多行。
3. 并发两次 `execute_check_in` 只发起 **1** 次 HTTP 请求（租约生效）。
4. 三来源同天优先级：`manual` 胜 `provider_response` 胜 `provider_rule` 胜 `balance_delta` 胜 `legacy_estimate`。
5. 无基线（首日）不产生推断证据（修 D5）。
6. `delta <= 0` 既不产生也不删除证据。
7. `reward_cents` 可由高置信度覆盖，反向不允许。
8. `current_streak`：最后证据日 `today-2` ⇒ 0；`today-1` ⇒ 保留；`today` ⇒ 累加。
9. `check_in_rate` 分母 = 当月已过天数。
10. 日桶/日期字符串只由后端 helper 产生；前端零处 `new Date('<YYYY-MM-DD>')`。
11. `quota_adjustment` 的变化不产生推断证据。
12. 失败尝试落库且**不**产生证据；`skipped_guard` 不计入失败率分母。

## 迁移与兼容（每步可独立发布）
| 步骤 | 内容 | 风险 |
|---|---|---|
| M1 | 新增 `check_in_attempts` + 索引 | 无（纯新增） |
| M2 | 证据加 `confidence`/`attempt_id`，回填并按 offset 重算 `quota_day` | 中（需 dry-run 对比受影响行数） |
| M3 | accounts 缓存列 + provider 配置列，回填历史 | 低 |
| M4 | 读路径切到缓存列；`can_check_in` → `check_in_state` | 中（行为变化需回归调度） |
| M5 | 上线运行时推断（事务包裹证据 + 缓存 + 快照） | 中（误报为已知代价） |
| M6 | 删除 `last_check_in` 读写与 DTO 字段；前端切新字段 | 低（`DROP COLUMN` 需确认 SQLite 版本） |
| M7 | 退避/single-flight + 历史/统计/jobs + `stop` 取消 | 高（新功能，需契约测试） |
| M8 | 清理死代码（见下） | 低 |

**必须同批修掉的前端问题**（否则后端修了也看不到效果）：三套 queryKey 命名（`['check-in']` / `['checkInStreak']` / `['check-in-streak']`）统一到 `checkInKeys.*`，并把 `refetchOnWindowFocus: false` 下的失效策略补齐。

**M8 死代码清单**：`check_in_jobs` 表 / `CheckInJob` 聚合 / `CheckInJobRepository` / `aggregate_test.rs`（13 测试）（或按 U2 复活）；`services/check_in_helpers.rs`（**整个文件未被 `mod` 声明，从未编译**）；`recalculate_all_streaks` 空实现 + IPC + 前端 hook；前端 8 个零引用组件/hook（`CheckInButton`、`BatchCheckInButton`、`StreakStatsCards`、`AccountBalanceDisplay`、`useSmartAccountBalance`、`useAllCheckInStreaks`、`useRecalculateStreaks`、`AccountDrawer`）。

## 后果

### 优点
- 守卫从「时间近似」变成「事实判定」，同时修掉重复签到与隔天退化。
- 网页签到（约束 1）成为**运行时**一等路径，且置信度对用户可见。
- `last_check_in` 的三义混淆消失；「最后签到」与「上次查询」不再重复。
- 尝试履历一次性解锁：退避、single-flight、历史、统计、取消、可观测。
- 全部不变式可写成纯函数/集成测试，streak 目录零测试的局面结束。

### 代价
- 多一张表 + 状态机；`check_in_records` 语义变更需要一次重算迁移。
- `quota_day` 依赖 provider offset 配置，配置缺失时回退 UTC（行为不回退，但时区收益也拿不到）。
- 推断仍有误报（充值/赠送）；缓解靠 UI 标注与撤销入口，不靠算法。

### 风险
- `DROP COLUMN` 与 `RENAME COLUMN` 依赖 SQLite 版本（bundled 版本需确认）。
- 行为变化集中在调度器：`skipped_guard`/`Backoff` 若统计口径不清会掩盖真实失败 → 用 `check_in_attempts` 的 outcome 区分。
- 迁移 M2 需在真实旧库上 dry-run（现有测试用 `include_str!` 手工重放迁移，绕过 `sqlx::migrate!`，`test_helpers.rs` 按 `;` 朴素切分，**无法**验证真实升级路径）。

## 未决问题
- **U1（原 Q6）**：`manual` 是否应无条件覆盖 `provider_response`？现状 `manual` 在 UPSERT 中胜出，但补记入口有前置检查：已结算（confirmed/manual 且有金额）的一天不会被覆盖，而覆盖推断值时会写一条 attempt 作为审计。**已按建议实现。**
- **U2**：`check_in_jobs` 复活还是删除？**已解决**：该表早在 `20250202000001_drop_unused_tables.sql` 就被删除，`CheckInJob` 聚合 / `CheckInJobRepository` / 其 13 个测试描述的是不存在的存储，已整体删除；运行中任务改为内存 `CheckInJobRegistry`，历史改读 `check_in_attempts`。详见「实施记录」。
- **U3**：推断证据是否允许用户「撤销该天」？**仍开放**。当前缓解是 `confidence='inferred'` 在 UI 可见（`reward_is_estimate`、账号表最后签到时间的 `*` 标记）。
- **U4**：provider `check_in_day_offset_minutes` 的来源？**已解决**：DB 列（默认 480），迁移与内置种子共用 `CheckInMode::infer` 同一规则。
- **U5**：`observe_only` 模式下是否需要**自动化**余额轮询？**仍开放**。运行时推断已上线，但触发点仍是一次余额观察（手动刷新、页面加载、签到流程）；调度器目前只跑签到任务。这决定「用户不打开 App 时网页签到能否被发现」。

## 实施记录（2026-09）

**迁移编号与本文的 M 编号不同**，按依赖顺序落地为：
`20260911000001_add_check_in_attempts` → `20260911000002_add_provider_check_in_config`（providers 的 offset/mode + accounts 的两个缓存列）→ `20260911000003_rebuild_check_in_evidence`（重建证据表：`confidence`、`quota_day`、`balance_delta`，按 offset 重分桶并按优先级去重）。00002 必须在前，因为重分桶要读 provider 的 offset；SQLite 无法修改 CHECK 约束，所以「让 `source` 接纳 `balance_delta`」只能靠重建表，重建顺带完成去重与重分桶。

**与原稿的偏差（实施中发现的更优解）**：
1. **`confidence` 由 `source` 派生**，写入端不单独传：`CheckInSource::confidence()` 是唯一映射，运行时 UPSERT 与重建迁移共用同一张优先级表，三者不可能互相矛盾。
2. **推断不做二次扣减**：`balance_history.total_quota` 在写入时**已经**扣过 `quota_adjustment`，所以运行时推断直接比较相邻 quota day 的存量值；再减一次会重复扣减。另加 `MIN_INFERRED_DELTA = 0.05` 作为舍入噪声下限（`total_quota` 由两个已舍入值推导，而真实奖励是整美元）。
3. **「已签到」响应成为一等证据**：provider 回「今天已签到 / 重复签到」不再算失败，而是 `already_checked_in` → 写一条 `confirmed_unknown` 证据。这比额度差分更可靠，也让重复请求不再显示为错误。
4. **HTTP 失败改为稳定错误码**：`check_in_codes`（`not_supported` / `http_status_client` / `http_status_server` / `waf_challenge` / `network` / `timeout` / `cancelled`）随结果返回，尝试结果由码映射，不再解析人类可读文案；上游响应体也不再进入错误消息与日志（只以截断形式进日志）。
5. **破坏性字段替换**：`accounts.last_check_in` 与其 DTO 字段被删除，改为 `last_confirmed_at` / `last_evidence_at`；前端账号表优先显示 confirmed，仅有推断时以 `*` 标记来源。
6. **补齐（D-7）落地形态**：`get_check_in_history`（服务端分页，返回尝试而非任务）、`get_check_in_stats`（按账号与周期：成功率分母只含已判定的尝试，skip 不稀释；含 confirmed/inferred 天数与奖励合计）、`get_check_in_state`（新命令，返回 `due/done/in_flight/backoff`）、`get_running_jobs` 与 `stop_check_in`（协作式取消：批量在账号之间检查标志，在途请求不中断）。前端历史/统计 UI 尚未消费这些命令。

**验证**：`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all --check`、`tsc --noEmit`、`vitest run` 全绿；测试覆盖「不变式」一节的第 1–12 条（domain 的 `balance_history::streak` / `check_in::state` / `check_in::quota_day`，infra 的 `check_in_reward_integration`，app 的 `check_in_executor::tests` 与 `check_in_streak_queries::tests`）。
