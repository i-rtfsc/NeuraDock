//! 日志系统模块
//!
//! 提供统一的日志记录功能，支持：
//! - 结构化 JSON 日志（生产环境）- One-line JSON 格式
//! - 人类可读彩色日志（开发环境）
//! - 日志文件轮转
//! - 前端日志上报
//!
//! 每条日志包含完整元数据：
//! - timestamp: ISO 8601 带时区，毫秒精度（例如 2025-12-09T10:32:15.123+08:00）
//! - level: TRACE/DEBUG/INFO/WARN/ERROR
//! - target: 模块名/TAG
//! - pid: 进程 ID
//! - tid: 线程 ID
//! - file + line: 源代码位置
//! - message: 主要文本
//! - fields: 结构化字段（key/value）
//! - source: backend / frontend
//! - version: 应用版本

use log::LevelFilter;
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::Level;
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Layer, Registry};

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();
static APP_VERSION: OnceLock<String> = OnceLock::new();
static LOGGER_READY: OnceLock<()> = OnceLock::new();
static FILE_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// 初始化日志系统
///
/// 在 debug 模式下：
/// - 输出到 stdout（彩色，人类可读）
/// - 输出到文件（JSON 格式，one-line）
///
/// 在 release 模式下：
/// - 仅输出到文件（JSON 格式，one-line）
pub fn init_logger(log_dir: PathBuf) -> anyhow::Result<()> {
    if LOGGER_READY.get().is_some() {
        return Ok(());
    }

    // 确保日志目录存在
    std::fs::create_dir_all(&log_dir)?;

    // 保存到全局静态变量供后续使用
    let _ = LOG_DIR.set(log_dir.clone());
    let _ = APP_VERSION.set(env!("CARGO_PKG_VERSION").to_string());

    // 将 log crate 的日志转发到 tracing
    let _ = LogTracer::builder()
        .with_max_level(LevelFilter::Trace)
        .init();

    // 创建文件 appender（按天轮转）
    let file_appender = rolling::daily(&log_dir, "neuradock.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = FILE_GUARD.set(guard);

    // JSON 层：写入文件（one-line JSON 格式）
    let json_layer = fmt::layer()
        .with_writer(non_blocking)
        .json()
        .with_current_span(false) // 不需要 span 列表，简化输出
        .with_span_list(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(true)
        .with_timer(fmt::time::ChronoLocal::new(
            "%Y-%m-%dT%H:%M:%S%.3f%:z".to_string(), // ISO 8601 带时区和毫秒
        ))
        .event_format(JsonFormatter::new())
        .with_filter(get_file_filter());

    // 人类可读层：输出到 stdout（仅 debug 模式）
    let stdout_layer = if cfg!(debug_assertions) {
        Some(
            fmt::layer()
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_ansi(true) // 彩色输出
                .with_timer(fmt::time::ChronoLocal::new(
                    "%Y-%m-%d %H:%M:%S%.3f".to_string(),
                ))
                .event_format(HumanReadableFormatter::new())
                .with_filter(get_stdout_filter()),
        )
    } else {
        None
    };

    // 组合所���层
    let subscriber = Registry::default().with(json_layer).with(stdout_layer);

    // 设置为全局默认
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| anyhow::anyhow!("Failed to set global subscriber: {}", e))?;

    let _ = LOGGER_READY.set(());

    // 记录初始化日志
    tracing::info!(
        target: "neuradock::logging",
        source = "backend",
        log_dir = %log_dir.display(),
        version = env!("CARGO_PKG_VERSION"),
        profile = if cfg!(debug_assertions) { "Debug" } else { "Release" },
        "Logger initialized successfully"
    );

    Ok(())
}

/// 获取文件日志的过滤器
fn get_file_filter() -> EnvFilter {
    // 默认 INFO 级别，可通过 RUST_LOG 环境变量覆盖
    // 生产环境：INFO 及以上
    // 开发环境：DEBUG 及以上
    let default_level = if cfg!(debug_assertions) {
        "debug,neuradock=trace"
    } else {
        "info,neuradock=info"
    };

    EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(default_level))
        .unwrap()
}

/// 获取 stdout 日志的过滤器
fn get_stdout_filter() -> EnvFilter {
    // 仅在 debug 模式下输出到 stdout
    // 默认 DEBUG 级别
    EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("debug,neuradock=trace"))
        .unwrap()
}

/// 获取日志目录路径
pub fn get_log_dir() -> Option<PathBuf> {
    LOG_DIR.get().cloned()
}

/// 前端日志结构
#[derive(Debug, serde::Deserialize)]
pub struct FrontendLog {
    pub level: String,
    pub target: String,
    pub message: String,
    #[serde(default)]
    pub fields: Option<serde_json::Value>,
}

/// 记录来自前端的日志
pub fn log_from_frontend(log: FrontendLog) {
    let target_val = log.target.clone();
    let message = log.message.as_str();
    let fields = log.fields.as_ref();

    match log.level.to_lowercase().as_str() {
        "error" => {
            tracing::error!(
                target: "frontend",
                source = "frontend",
                frontend_target = %target_val,
                fields = ?fields,
                "{}",
                message
            );
        }
        "warn" => {
            tracing::warn!(
                target: "frontend",
                source = "frontend",
                frontend_target = %target_val,
                fields = ?fields,
                "{}",
                message
            );
        }
        "info" => {
            tracing::info!(
                target: "frontend",
                source = "frontend",
                frontend_target = %target_val,
                fields = ?fields,
                "{}",
                message
            );
        }
        "debug" => {
            tracing::debug!(
                target: "frontend",
                source = "frontend",
                frontend_target = %target_val,
                fields = ?fields,
                "{}",
                message
            );
        }
        "trace" => {
            tracing::trace!(
                target: "frontend",
                source = "frontend",
                frontend_target = %target_val,
                fields = ?fields,
                "{}",
                message
            );
        }
        _ => {
            tracing::info!(
                target: "frontend",
                source = "frontend",
                frontend_target = %target_val,
                fields = ?fields,
                "{}",
                message
            );
        }
    }
}

// ============================================================
// 自定义格式化器
// ============================================================

use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::{format::Writer, FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

/// JSON 格式化器 - One-line JSON
struct JsonFormatter {
    pid: u32,
    version: String,
}

impl JsonFormatter {
    fn new() -> Self {
        Self {
            pid: std::process::id(),
            version: APP_VERSION
                .get()
                .cloned()
                .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
        }
    }
}

impl<S, N> FormatEvent<S, N> for JsonFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let metadata = event.metadata();

        // 获取当前时间（ISO 8601 带时区和毫秒）
        let now = chrono::Local::now();
        let timestamp = now.format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string();

        // 获取线程信息
        let thread = std::thread::current();
        let tid = format!("{:?}", thread.id());
        let thread_name = thread.name().unwrap_or("unnamed");

        // 构建 JSON 对象
        let mut json = serde_json::json!({
            "timestamp": timestamp,
            "level": metadata.level().to_string(),
            "source": "backend", // 默认 backend
            "pid": self.pid,
            "tid": tid,
            "thread_name": thread_name,
            "target": metadata.target(),
            "version": self.version,
        });

        // 添加文件和行号（如果有）
        if let Some(file) = metadata.file() {
            json["file"] = serde_json::json!(file);
        }
        if let Some(line) = metadata.line() {
            json["line"] = serde_json::json!(line);
        }

        // 收集字段（包括 message 和其他自定义字段）
        let mut fields_visitor = JsonVisitor::new();
        event.record(&mut fields_visitor);

        // 提取 message
        if let Some(message) = fields_visitor.fields.get("message") {
            json["message"] = message.clone();
        }

        // 添加其他字段（排除 message）
        let mut custom_fields = serde_json::Map::new();
        for (key, value) in fields_visitor.fields.iter() {
            if key != "message" {
                custom_fields.insert(key.clone(), value.clone());
            }
        }

        // 如果有 source 字段，覆盖默认值
        if let Some(source) = custom_fields.get("source") {
            json["source"] = source.clone();
            custom_fields.remove("source");
        }

        if !custom_fields.is_empty() {
            json["fields"] = serde_json::json!(custom_fields);
        }

        // 写入 one-line JSON
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&json).unwrap_or_default()
        )
    }
}

/// 人类可读格式化器
/// 格式：2025-12-09 10:32:15.123 [INFO] (auth::login) pid=12345 tid=thread-7 user_id=42 — message (src/auth.rs:128)
struct HumanReadableFormatter {
    pid: u32,
}

impl HumanReadableFormatter {
    fn new() -> Self {
        Self {
            pid: std::process::id(),
        }
    }
}

impl<S, N> FormatEvent<S, N> for HumanReadableFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let metadata = event.metadata();

        // 时间戳
        let now = chrono::Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        // 线程信息
        let thread = std::thread::current();
        let tid = format!("{:?}", thread.id());

        // 级别（带颜色）
        let level = metadata.level();
        let level_str = match *level {
            Level::ERROR => "\x1b[31mERROR\x1b[0m", // 红色
            Level::WARN => "\x1b[33mWARN\x1b[0m",   // 黄色
            Level::INFO => "\x1b[32mINFO\x1b[0m",   // 绿色
            Level::DEBUG => "\x1b[36mDEBUG\x1b[0m", // 青色
            Level::TRACE => "\x1b[35mTRACE\x1b[0m", // 紫色
        };

        // 收集字段
        let mut fields_visitor = JsonVisitor::new();
        event.record(&mut fields_visitor);

        let message = fields_visitor
            .fields
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // 构建字段字符串
        let mut field_parts = Vec::new();
        for (key, value) in fields_visitor.fields.iter() {
            if key != "message" && key != "source" {
                field_parts.push(format!("{}={}", key, value));
            }
        }
        let fields_str = if field_parts.is_empty() {
            String::new()
        } else {
            format!(" {}", field_parts.join(" "))
        };

        // 文件和行号
        let location = if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
            format!(" ({}:{})", file, line)
        } else {
            String::new()
        };

        // 输出格式：timestamp [LEVEL] (target) pid=xxx tid=xxx fields — message (file:line)
        writeln!(
            writer,
            "{} [{}] ({}) pid={} tid={}{} — {}{}",
            timestamp,
            level_str,
            metadata.target(),
            self.pid,
            tid,
            fields_str,
            message,
            location
        )
    }
}

/// 访问者模式收集事件字段
struct JsonVisitor {
    fields: serde_json::Map<String, serde_json::Value>,
}

impl JsonVisitor {
    fn new() -> Self {
        Self {
            fields: serde_json::Map::new(),
        }
    }
}

impl tracing::field::Visit for JsonVisitor {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::json!(format!("{:?}", value)),
        );
    }
}
