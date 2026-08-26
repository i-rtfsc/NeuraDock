use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use super::server::{GatewayClient, GatewayUpstream};
use crate::fs_safe::atomic_write;

const FAILURE_THRESHOLD: u32 = 3;
const RECOVERY_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct GatewayTokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
}

impl GatewayTokenUsage {
    pub fn merge(&mut self, other: Self) {
        self.input_tokens = self.input_tokens.max(other.input_tokens);
        self.output_tokens = self.output_tokens.max(other.output_tokens);
        self.cache_read_tokens = self.cache_read_tokens.max(other.cache_read_tokens);
        self.cache_creation_tokens = self.cache_creation_tokens.max(other.cache_creation_tokens);
    }
}

#[derive(Debug, Clone)]
pub struct GatewayCandidate {
    pub upstream: GatewayUpstream,
    pub half_open_permit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayUpstreamSnapshot {
    pub id: String,
    pub label: String,
    pub state: CircuitState,
    pub consecutive_failures: u32,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub usage: GatewayTokenUsage,
    pub average_latency_ms: u64,
    pub average_first_event_latency_ms: u64,
    pub last_error: Option<String>,
    pub last_error_at_ms: Option<u64>,
    pub recovery_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeEntry {
    label: String,
    state: CircuitState,
    consecutive_failures: u32,
    #[serde(default)]
    half_open_in_flight: bool,
    opened_at_ms: Option<u64>,
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    usage: GatewayTokenUsage,
    total_latency_ms: u128,
    #[serde(default)]
    total_first_event_latency_ms: u128,
    #[serde(default)]
    requests_with_first_event: u64,
    last_error: Option<String>,
    last_error_at_ms: Option<u64>,
}

impl RuntimeEntry {
    fn new(label: String) -> Self {
        Self {
            label,
            state: CircuitState::Closed,
            consecutive_failures: 0,
            half_open_in_flight: false,
            opened_at_ms: None,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            usage: GatewayTokenUsage::default(),
            total_latency_ms: 0,
            total_first_event_latency_ms: 0,
            requests_with_first_event: 0,
            last_error: None,
            last_error_at_ms: None,
        }
    }

    fn refresh_state(&mut self) {
        if self.state == CircuitState::Open
            && self
                .opened_at_ms
                .map(|opened| {
                    now_ms().saturating_sub(opened) >= RECOVERY_TIMEOUT.as_millis() as u64
                })
                .unwrap_or(false)
        {
            self.state = CircuitState::HalfOpen;
            self.half_open_in_flight = false;
        }
    }

    fn snapshot(&self, id: String) -> GatewayUpstreamSnapshot {
        GatewayUpstreamSnapshot {
            id,
            label: self.label.clone(),
            state: self.state,
            consecutive_failures: self.consecutive_failures,
            total_requests: self.total_requests,
            successful_requests: self.successful_requests,
            failed_requests: self.failed_requests,
            usage: self.usage,
            average_latency_ms: if self.total_requests == 0 {
                0
            } else {
                (self.total_latency_ms / self.total_requests as u128) as u64
            },
            average_first_event_latency_ms: if self.requests_with_first_event == 0 {
                0
            } else {
                (self.total_first_event_latency_ms / self.requests_with_first_event as u128) as u64
            },
            last_error: self.last_error.clone(),
            last_error_at_ms: self.last_error_at_ms,
            recovery_at_ms: if self.state == CircuitState::Open {
                self.opened_at_ms
                    .map(|opened| opened.saturating_add(RECOVERY_TIMEOUT.as_millis() as u64))
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedRuntimeEntry {
    client: GatewayClient,
    upstream_id: String,
    entry: RuntimeEntry,
}

pub struct GatewayRuntime {
    entries: Mutex<HashMap<(GatewayClient, String), RuntimeEntry>>,
    persistence_path: RwLock<Option<PathBuf>>,
}

impl Default for GatewayRuntime {
    fn default() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            persistence_path: RwLock::new(None),
        }
    }
}

impl GatewayRuntime {
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub async fn configure_persistence(&self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        *self.persistence_path.write().unwrap() = Some(path.clone());
        let Ok(content) = std::fs::read_to_string(&path) else {
            return;
        };
        match serde_json::from_str::<Vec<PersistedRuntimeEntry>>(&content) {
            Ok(saved) => {
                let mut entries = self.entries.lock().await;
                for saved_entry in saved {
                    entries.insert(
                        (saved_entry.client, saved_entry.upstream_id),
                        saved_entry.entry,
                    );
                }
            }
            Err(error) => log::warn!("Failed to load gateway request statistics: {}", error),
        }
    }

    fn persist(&self, entries: &HashMap<(GatewayClient, String), RuntimeEntry>) {
        let Some(path) = self.persistence_path.read().unwrap().clone() else {
            return;
        };
        let saved: Vec<PersistedRuntimeEntry> = entries
            .iter()
            .map(|((client, upstream_id), entry)| PersistedRuntimeEntry {
                client: *client,
                upstream_id: upstream_id.clone(),
                entry: entry.clone(),
            })
            .collect();
        let result = serde_json::to_string_pretty(&saved)
            .map_err(anyhow::Error::from)
            .and_then(|json| atomic_write(&path, &json, false));
        if let Err(error) = result {
            log::warn!("Failed to persist gateway request statistics: {}", error);
        }
    }

    pub async fn candidate(
        &self,
        client: GatewayClient,
        upstream: &GatewayUpstream,
    ) -> Option<GatewayCandidate> {
        let mut entries = self.entries.lock().await;
        let entry = entries
            .entry((client, upstream.id.clone()))
            .or_insert_with(|| RuntimeEntry::new(upstream.label.clone()));
        entry.label = upstream.label.clone();
        entry.refresh_state();

        let half_open_permit = match entry.state {
            CircuitState::Closed => false,
            CircuitState::Open => return None,
            CircuitState::HalfOpen if entry.half_open_in_flight => return None,
            CircuitState::HalfOpen => {
                entry.half_open_in_flight = true;
                true
            }
        };
        Some(GatewayCandidate {
            upstream: upstream.clone(),
            half_open_permit,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn record_success(
        &self,
        client: GatewayClient,
        upstream: &GatewayUpstream,
        half_open_permit: bool,
        usage: GatewayTokenUsage,
        latency: Duration,
        first_event_latency: Option<Duration>,
    ) {
        let mut entries = self.entries.lock().await;
        let entry = entries
            .entry((client, upstream.id.clone()))
            .or_insert_with(|| RuntimeEntry::new(upstream.label.clone()));
        entry.label = upstream.label.clone();
        entry.total_requests += 1;
        entry.successful_requests += 1;
        entry.total_latency_ms += latency.as_millis();
        if let Some(first_event_latency) = first_event_latency {
            entry.total_first_event_latency_ms += first_event_latency.as_millis();
            entry.requests_with_first_event += 1;
        }
        entry.usage.input_tokens += usage.input_tokens;
        entry.usage.output_tokens += usage.output_tokens;
        entry.usage.cache_read_tokens += usage.cache_read_tokens;
        entry.usage.cache_creation_tokens += usage.cache_creation_tokens;
        entry.consecutive_failures = 0;
        entry.last_error = None;
        if half_open_permit || entry.state == CircuitState::HalfOpen {
            entry.state = CircuitState::Closed;
            entry.opened_at_ms = None;
        }
        entry.half_open_in_flight = false;
        self.persist(&entries);
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn record_failure(
        &self,
        client: GatewayClient,
        upstream: &GatewayUpstream,
        half_open_permit: bool,
        latency: Duration,
        affects_circuit: bool,
        error: String,
    ) {
        let mut entries = self.entries.lock().await;
        let entry = entries
            .entry((client, upstream.id.clone()))
            .or_insert_with(|| RuntimeEntry::new(upstream.label.clone()));
        entry.label = upstream.label.clone();
        entry.total_requests += 1;
        entry.failed_requests += 1;
        entry.total_latency_ms += latency.as_millis();
        entry.last_error = Some(error);
        entry.last_error_at_ms = Some(now_ms());
        entry.half_open_in_flight = false;

        if !affects_circuit {
            self.persist(&entries);
            return;
        }
        entry.consecutive_failures += 1;
        if half_open_permit
            || entry.state == CircuitState::HalfOpen
            || entry.consecutive_failures >= FAILURE_THRESHOLD
        {
            entry.state = CircuitState::Open;
            entry.opened_at_ms = Some(now_ms());
        }
        self.persist(&entries);
    }

    pub async fn snapshot(
        &self,
        client: GatewayClient,
        upstream: &GatewayUpstream,
    ) -> GatewayUpstreamSnapshot {
        let mut entries = self.entries.lock().await;
        let entry = entries
            .entry((client, upstream.id.clone()))
            .or_insert_with(|| RuntimeEntry::new(upstream.label.clone()));
        entry.label = upstream.label.clone();
        entry.refresh_state();
        entry.snapshot(upstream.id.clone())
    }

    pub async fn release_neutral(&self, client: GatewayClient, upstream_id: &str) {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.get_mut(&(client, upstream_id.to_string())) {
            entry.half_open_in_flight = false;
        }
        self.persist(&entries);
    }

    pub async fn reset(&self, client: GatewayClient) {
        let mut entries = self.entries.lock().await;
        entries.retain(|(entry_client, _), _| *entry_client != client);
        self.persist(&entries);
    }

    pub async fn recover(&self, client: GatewayClient, upstream_id: &str) {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.get_mut(&(client, upstream_id.to_string())) {
            entry.state = CircuitState::Closed;
            entry.consecutive_failures = 0;
            entry.half_open_in_flight = false;
            entry.opened_at_ms = None;
        }
        self.persist(&entries);
    }

    pub async fn clear_statistics(&self, client: GatewayClient, upstream_id: &str) {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.get_mut(&(client, upstream_id.to_string())) {
            let label = entry.label.clone();
            *entry = RuntimeEntry::new(label);
        }
        self.persist(&entries);
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn usage_from_json(value: &serde_json::Value) -> GatewayTokenUsage {
    let Some(usage) = value.get("usage") else {
        return GatewayTokenUsage::default();
    };
    GatewayTokenUsage {
        input_tokens: usage
            .get("input_tokens")
            .or_else(|| usage.get("prompt_tokens"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        output_tokens: usage
            .get("output_tokens")
            .or_else(|| usage.get("completion_tokens"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        cache_read_tokens: usage
            .get("cache_read_input_tokens")
            .or_else(|| usage.pointer("/input_tokens_details/cached_tokens"))
            .or_else(|| usage.pointer("/prompt_tokens_details/cached_tokens"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        cache_creation_tokens: usage
            .get("cache_creation_input_tokens")
            .or_else(|| usage.pointer("/input_tokens_details/cache_write_tokens"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
    }
}

pub(crate) fn circuit_failure_status(status: StatusCode) -> bool {
    status.is_server_error()
        || matches!(
            status,
            StatusCode::REQUEST_TIMEOUT | StatusCode::TOO_MANY_REQUESTS
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::UpstreamKind;

    fn upstream(id: &str) -> GatewayUpstream {
        GatewayUpstream {
            id: id.to_string(),
            label: id.to_string(),
            base_url: "https://example.com".to_string(),
            api_key: "test".to_string(),
            kind: UpstreamKind::OpenAiChat,
            model: None,
            impersonate: false,
        }
    }

    #[tokio::test]
    async fn opens_after_consecutive_failures_and_blocks_requests() {
        let runtime = GatewayRuntime::default();
        let primary = upstream("primary");

        for _ in 0..FAILURE_THRESHOLD {
            runtime
                .record_failure(
                    GatewayClient::Codex,
                    &primary,
                    false,
                    Duration::from_millis(1),
                    true,
                    "failed".to_string(),
                )
                .await;
        }

        assert!(runtime
            .candidate(GatewayClient::Codex, &primary)
            .await
            .is_none());
    }

    #[tokio::test]
    async fn half_open_allows_only_one_probe() {
        let runtime = GatewayRuntime::default();
        let primary = upstream("primary");
        let mut entry = RuntimeEntry::new(primary.label.clone());
        entry.state = CircuitState::Open;
        entry.opened_at_ms = Some(now_ms().saturating_sub(RECOVERY_TIMEOUT.as_millis() as u64));
        runtime
            .entries
            .lock()
            .await
            .insert((GatewayClient::Codex, primary.id.clone()), entry);

        let first = runtime
            .candidate(GatewayClient::Codex, &primary)
            .await
            .unwrap();
        let second = runtime.candidate(GatewayClient::Codex, &primary).await;

        assert_eq!(first.upstream.id, "primary");
        assert!(first.half_open_permit);
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn manual_recovery_closes_the_circuit_without_sending_a_request() {
        let runtime = GatewayRuntime::default();
        let primary = upstream("primary");
        for _ in 0..FAILURE_THRESHOLD {
            runtime
                .record_failure(
                    GatewayClient::Codex,
                    &primary,
                    false,
                    Duration::from_millis(1),
                    true,
                    "failed".to_string(),
                )
                .await;
        }

        runtime.recover(GatewayClient::Codex, &primary.id).await;
        let snapshot = runtime.snapshot(GatewayClient::Codex, &primary).await;
        assert_eq!(snapshot.state, CircuitState::Closed);
        assert_eq!(snapshot.consecutive_failures, 0);
        assert_eq!(snapshot.total_requests, FAILURE_THRESHOLD as u64);
    }

    #[tokio::test]
    async fn statistics_persist_and_can_be_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("runtime.json");
        let primary = upstream("primary");
        let runtime = GatewayRuntime::default();
        runtime.configure_persistence(&path).await;
        runtime
            .record_success(
                GatewayClient::ClaudeCode,
                &primary,
                false,
                GatewayTokenUsage {
                    input_tokens: 10,
                    output_tokens: 4,
                    ..Default::default()
                },
                Duration::from_millis(20),
                Some(Duration::from_millis(5)),
            )
            .await;

        let restored = GatewayRuntime::default();
        restored.configure_persistence(&path).await;
        let snapshot = restored.snapshot(GatewayClient::ClaudeCode, &primary).await;
        assert_eq!(snapshot.total_requests, 1);
        assert_eq!(snapshot.usage.input_tokens, 10);
        assert_eq!(snapshot.average_first_event_latency_ms, 5);

        restored
            .clear_statistics(GatewayClient::ClaudeCode, &primary.id)
            .await;
        let cleared = restored.snapshot(GatewayClient::ClaudeCode, &primary).await;
        assert_eq!(cleared.total_requests, 0);
        assert_eq!(cleared.usage.input_tokens, 0);
    }

    #[test]
    fn parses_common_usage_shapes() {
        let usage = usage_from_json(&serde_json::json!({
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 4,
                "prompt_tokens_details": { "cached_tokens": 3 }
            }
        }));
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 4);
        assert_eq!(usage.cache_read_tokens, 3);

        let anthropic = usage_from_json(&serde_json::json!({
            "usage": {
                "input_tokens": 20,
                "output_tokens": 8,
                "cache_read_input_tokens": 6,
                "cache_creation_input_tokens": 2
            }
        }));
        assert_eq!(anthropic.input_tokens, 20);
        assert_eq!(anthropic.output_tokens, 8);
        assert_eq!(anthropic.cache_read_tokens, 6);
        assert_eq!(anthropic.cache_creation_tokens, 2);

        let responses = usage_from_json(&serde_json::json!({
            "usage": {
                "input_tokens": 12,
                "output_tokens": 5,
                "input_tokens_details": { "cached_tokens": 4 }
            }
        }));
        assert_eq!(responses.input_tokens, 12);
        assert_eq!(responses.output_tokens, 5);
        assert_eq!(responses.cache_read_tokens, 4);
    }
}
