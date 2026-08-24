//! Agent access records: a single source of truth for "which key each agent
//! is currently configured with, and how" (direct write vs. via the gateway).
//!
//! This backs the Agent 配置 page's status cards. Both the direct-write config
//! commands and the gateway binding commands record here, so the UI can show a
//! consistent picture regardless of the method used. The real key is never
//! stored; only a human label and a reference are kept.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::presentation::error::CommandError;
use neuradock_infrastructure::fs_safe::atomic_write;

const ACCESS_FILENAME: &str = "agent_access.json";

/// Which agent a record applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum AgentIdDto {
    Codex,
    ClaudeCode,
}

/// How the agent is wired to its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum AccessMethodDto {
    /// Key written straight into the tool config file (no gateway).
    Direct,
    /// Requests go through the local adaptation gateway.
    Gateway,
}

/// Structured reference to the configured key. This intentionally contains no
/// secret material; it only carries enough identity to restore the editor.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentAccessSourceDto {
    AccountToken {
        account_id: String,
        provider_id: String,
        token_id: i64,
        base_url: Option<String>,
    },
    IndependentKey {
        key_id: i64,
    },
}

/// One agent's current access record. `upstream_kind` / `impersonate` only
/// apply to the gateway method.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct AgentAccessDto {
    pub agent: AgentIdDto,
    pub method: AccessMethodDto,
    /// Human-readable key description (name, never the key material).
    pub label: String,
    #[serde(default)]
    pub account_label: Option<String>,
    pub model: Option<String>,
    pub upstream_kind: Option<String>,
    pub impersonate: Option<bool>,
    /// Added after the first access-record format shipped. Old records remain
    /// readable and simply require the user to select the key once more.
    #[serde(default)]
    pub source: Option<AgentAccessSourceDto>,
    #[serde(default)]
    pub context1m: Option<bool>,
    /// For Codex direct/gateway bindings written to an isolated file instead of
    /// the official config.toml/auth.json: the provider slug the file is named
    /// after (`profiles.d/<slug>.config.toml`). `None` means the official files are used.
    #[serde(default)]
    pub isolated_slug: Option<String>,
}

/// In-memory cache of access records, mirrored to disk.
#[derive(Default)]
pub struct AgentAccessState {
    records: Mutex<HashMap<AgentIdDto, AgentAccessDto>>,
    profiles: Mutex<HashMap<String, AgentAccessDto>>,
}

fn profile_key(record: &AgentAccessDto) -> String {
    format!(
        "{:?}:{}",
        record.agent,
        record.isolated_slug.as_deref().unwrap_or("default")
    )
}

impl AgentAccessState {
    fn path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
        app.path()
            .app_data_dir()
            .ok()
            .map(|d| d.join(ACCESS_FILENAME))
    }

    fn persist(&self, app: &tauri::AppHandle) -> Result<(), CommandError> {
        let path = Self::path(app)
            .ok_or_else(|| CommandError::infrastructure("Failed to get app data directory"))?;
        let profiles = self.profiles.lock().unwrap();
        let list: Vec<&AgentAccessDto> = profiles.values().collect();
        let json = serde_json::to_string_pretty(&list)
            .map_err(|error| CommandError::infrastructure(error.to_string()))?;
        atomic_write(&path, &json, false)
            .map_err(|error| CommandError::infrastructure(error.to_string()))
    }

    /// Load records from disk into memory (call once at startup).
    pub fn load(&self, app: &tauri::AppHandle) {
        let Some(path) = Self::path(app) else { return };
        let Ok(content) = std::fs::read_to_string(&path) else {
            return;
        };
        match serde_json::from_str::<Vec<AgentAccessDto>>(&content) {
            Ok(list) => {
                let mut records = self.records.lock().unwrap();
                let mut profiles = self.profiles.lock().unwrap();
                for r in list {
                    profiles.insert(profile_key(&r), r.clone());
                    records.insert(r.agent, r);
                }
            }
            Err(e) => log::warn!("Failed to parse agent access records: {}", e),
        }
    }

    /// Upsert a record (used internally by the gateway commands too).
    pub fn set(&self, app: &tauri::AppHandle, record: AgentAccessDto) -> Result<(), CommandError> {
        let agent = record.agent;
        let key = profile_key(&record);
        let previous = self.records.lock().unwrap().insert(agent, record.clone());
        let previous_profile = self.profiles.lock().unwrap().insert(key.clone(), record);
        if let Err(error) = self.persist(app) {
            let mut records = self.records.lock().unwrap();
            let mut profiles = self.profiles.lock().unwrap();
            match previous {
                Some(previous) => {
                    records.insert(agent, previous);
                }
                None => {
                    records.remove(&agent);
                }
            }
            match previous_profile {
                Some(previous) => {
                    profiles.insert(key, previous);
                }
                None => {
                    profiles.remove(&key);
                }
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn get(&self, agent: AgentIdDto) -> Option<AgentAccessDto> {
        self.records.lock().unwrap().get(&agent).cloned()
    }

    pub fn list(&self, agent: AgentIdDto) -> Vec<AgentAccessDto> {
        self.profiles
            .lock()
            .unwrap()
            .values()
            .filter(|record| record.agent == agent)
            .cloned()
            .collect()
    }

    /// Remove a record.
    pub fn remove(&self, app: &tauri::AppHandle, agent: AgentIdDto) -> Result<(), CommandError> {
        let previous = self.records.lock().unwrap().remove(&agent);
        let previous_profiles: Vec<(String, AgentAccessDto)> = {
            let mut profiles = self.profiles.lock().unwrap();
            let keys = profiles
                .iter()
                .filter(|(_, record)| record.agent == agent)
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>();
            keys.into_iter()
                .filter_map(|key| profiles.remove(&key).map(|record| (key, record)))
                .collect()
        };
        if let Err(error) = self.persist(app) {
            if let Some(previous) = previous {
                self.records.lock().unwrap().insert(agent, previous);
            }
            self.profiles.lock().unwrap().extend(previous_profiles);
            return Err(error);
        }
        Ok(())
    }
}

/// Record an agent access (called by the frontend after a successful
/// direct-write configure, and internally after a gateway bind).
#[tauri::command]
#[specta::specta]
pub async fn record_agent_access(
    record: AgentAccessDto,
    app: tauri::AppHandle,
    state: State<'_, AgentAccessState>,
    gateway: State<'_, super::gateway::GatewayState>,
) -> Result<Vec<AgentAccessDto>, CommandError> {
    // Switching to direct write must retire any saved gateway binding, or a
    // later gateway start would take the tool config back over.
    if record.method == AccessMethodDto::Direct {
        let client = super::gateway::client_for_agent(record.agent);
        super::gateway::drop_gateway_binding_for_direct(&app, &gateway, client.into()).await?;
    }
    state.set(&app, record)?;
    Ok(list_records(&state))
}

/// Remove an agent's access record (called on clear / unbind).
#[tauri::command]
#[specta::specta]
pub async fn clear_agent_access(
    agent: AgentIdDto,
    app: tauri::AppHandle,
    state: State<'_, AgentAccessState>,
) -> Result<Vec<AgentAccessDto>, CommandError> {
    state.remove(&app, agent)?;
    Ok(list_records(&state))
}

/// All current access records for the status cards.
#[tauri::command]
#[specta::specta]
pub async fn get_agent_access(
    state: State<'_, AgentAccessState>,
) -> Result<Vec<AgentAccessDto>, CommandError> {
    Ok(list_records(&state))
}

fn list_records(state: &AgentAccessState) -> Vec<AgentAccessDto> {
    state.profiles.lock().unwrap().values().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_access_records_remain_deserializable() {
        let json = r#"{
            "agent": "codex",
            "method": "direct",
            "label": "Relay · Key",
            "model": null,
            "upstream_kind": null,
            "impersonate": null
        }"#;

        let record: AgentAccessDto = serde_json::from_str(json).unwrap();
        assert!(record.source.is_none());
        assert!(record.context1m.is_none());
    }
}
