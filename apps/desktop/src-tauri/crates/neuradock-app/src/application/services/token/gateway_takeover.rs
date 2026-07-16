//! Gateway takeover: snapshot a tool's live config, point it at the local
//! gateway with a placeholder key, and restore the snapshot afterwards.
//!
//! Mirrors cc-switch's takeover model: the real relay key never reaches the
//! tool's config file while the gateway is on, and stopping the gateway (or
//! the next app start after a crash) puts the original file back exactly.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use super::{ClaudeConfigService, CodexConfigService};
use neuradock_infrastructure::fs_safe::atomic_write;
use neuradock_infrastructure::gateway::GatewayClient;

/// Placeholder key written into tool configs during takeover; the real key
/// only lives inside the gateway process.
const GATEWAY_PLACEHOLDER_KEY: &str = "sk-neuradock-gateway";

const STATE_FILENAME: &str = "gateway_takeover.json";
const SNAPSHOT_DIR: &str = "gateway_takeover";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct TakeoverState {
    codex: Option<ClientSnapshot>,
    claude_code: Option<ClientSnapshot>,
}

/// Per-client record of which managed files existed when the snapshot was
/// taken; files that did not exist are deleted on restore instead of copied
/// back.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClientSnapshot {
    files: Vec<SnapshotFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotFile {
    /// Live path of the tool config file.
    live_path: PathBuf,
    /// Snapshot file name inside the snapshot directory, when the live file
    /// existed at takeover time.
    snapshot_name: Option<String>,
    /// Restore with owner-only permissions (files that carry key material).
    private: bool,
}

/// Manages takeover snapshots under the app data directory.
pub struct GatewayTakeover {
    state_path: PathBuf,
    snapshot_dir: PathBuf,
}

impl GatewayTakeover {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            state_path: app_data_dir.join(STATE_FILENAME),
            snapshot_dir: app_data_dir.join(SNAPSHOT_DIR),
        }
    }

    /// Whether any client is currently taken over (e.g. after a crash).
    pub fn has_stale_takeover(&self) -> bool {
        self.state_path.exists()
    }

    /// Take over `client`'s tool config: snapshot the live files, then write a
    /// config pointing at `gateway_base_url` with a placeholder key.
    pub fn take_over(&self, client: GatewayClient, gateway_base_url: &str) -> Result<()> {
        let mut state = self.load_state();
        if self.client_snapshot(&state, client).is_some() {
            // Already taken over: just refresh the placeholder config so a
            // new port or binding lands in the tool config.
            self.write_placeholder(client, gateway_base_url)?;
            return Ok(());
        }

        let snapshot = self.snapshot_files(client)?;
        *self.client_snapshot_mut(&mut state, client) = Some(snapshot);
        self.save_state(&state)?;

        self.write_placeholder(client, gateway_base_url)?;
        log::info!(
            "Gateway takeover active for {} -> {}",
            client.display_name(),
            gateway_base_url
        );
        Ok(())
    }

    /// Restore `client`'s tool config from its snapshot, if taken over.
    pub fn restore(&self, client: GatewayClient) -> Result<()> {
        let mut state = self.load_state();
        let Some(snapshot) = self.client_snapshot_mut(&mut state, client).take() else {
            return Ok(());
        };

        for file in &snapshot.files {
            match &file.snapshot_name {
                Some(name) => {
                    let content = fs::read_to_string(self.snapshot_dir.join(name))
                        .with_context(|| format!("Failed to read snapshot {}", name))?;
                    atomic_write(&file.live_path, &content, file.private)?;
                    let _ = fs::remove_file(self.snapshot_dir.join(name));
                }
                None => {
                    // The file did not exist before takeover.
                    let _ = fs::remove_file(&file.live_path);
                }
            }
        }

        self.save_state(&state)?;
        log::info!("Gateway takeover restored for {}", client.display_name());
        Ok(())
    }

    /// Restore every taken-over client (used on stop, exit, and the startup
    /// self-heal after a crash).
    pub fn restore_all(&self) -> Result<()> {
        for client in [GatewayClient::Codex, GatewayClient::ClaudeCode] {
            self.restore(client)?;
        }
        Ok(())
    }

    fn snapshot_files(&self, client: GatewayClient) -> Result<ClientSnapshot> {
        fs::create_dir_all(&self.snapshot_dir)
            .with_context(|| format!("Failed to create {}", self.snapshot_dir.display()))?;

        let targets: Vec<(PathBuf, &str, bool)> = match client {
            GatewayClient::Codex => {
                let (config, auth) = CodexConfigService::managed_paths()?;
                vec![
                    (config, "codex_config.toml", false),
                    (auth, "codex_auth.json", true),
                ]
            }
            GatewayClient::ClaudeCode => {
                let settings = ClaudeConfigService::managed_path()?;
                vec![(settings, "claude_settings.json", true)]
            }
        };

        let mut files = Vec::new();
        for (live_path, name, private) in targets {
            let snapshot_name = if live_path.exists() {
                fs::copy(&live_path, self.snapshot_dir.join(name)).with_context(|| {
                    format!("Failed to snapshot {}", live_path.display())
                })?;
                Some(name.to_string())
            } else {
                None
            };
            files.push(SnapshotFile {
                live_path,
                snapshot_name,
                private,
            });
        }
        Ok(ClientSnapshot { files })
    }

    fn write_placeholder(&self, client: GatewayClient, gateway_base_url: &str) -> Result<()> {
        match client {
            GatewayClient::Codex => {
                CodexConfigService::new().configure_global_with_key(
                    GATEWAY_PLACEHOLDER_KEY,
                    gateway_base_url,
                    None,
                    false,
                )?;
            }
            GatewayClient::ClaudeCode => {
                ClaudeConfigService::new().configure_global_with_key(
                    GATEWAY_PLACEHOLDER_KEY,
                    gateway_base_url,
                    None,
                    false,
                )?;
            }
        }
        Ok(())
    }

    fn client_snapshot<'a>(
        &self,
        state: &'a TakeoverState,
        client: GatewayClient,
    ) -> Option<&'a ClientSnapshot> {
        match client {
            GatewayClient::Codex => state.codex.as_ref(),
            GatewayClient::ClaudeCode => state.claude_code.as_ref(),
        }
    }

    fn client_snapshot_mut<'a>(
        &self,
        state: &'a mut TakeoverState,
        client: GatewayClient,
    ) -> &'a mut Option<ClientSnapshot> {
        match client {
            GatewayClient::Codex => &mut state.codex,
            GatewayClient::ClaudeCode => &mut state.claude_code,
        }
    }

    fn load_state(&self) -> TakeoverState {
        fs::read_to_string(&self.state_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    fn save_state(&self, state: &TakeoverState) -> Result<()> {
        if state.codex.is_none() && state.claude_code.is_none() {
            let _ = fs::remove_file(&self.state_path);
            let _ = fs::remove_dir_all(&self.snapshot_dir);
            return Ok(());
        }
        atomic_write(&self.state_path, &serde_json::to_string_pretty(state)?, false)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The snapshot/restore mechanics are exercised against temp dirs; the
    // placeholder write itself touches the real ~/.codex and is covered by
    // the config service tests, so these tests focus on state bookkeeping.

    #[test]
    fn state_round_trips_and_clears_when_empty() {
        let dir = tempfile::tempdir().unwrap();
        let takeover = GatewayTakeover::new(dir.path());

        let mut state = TakeoverState::default();
        state.codex = Some(ClientSnapshot {
            files: vec![SnapshotFile {
                live_path: dir.path().join("config.toml"),
                snapshot_name: Some("codex_config.toml".to_string()),
                private: false,
            }],
        });
        takeover.save_state(&state).unwrap();
        assert!(takeover.has_stale_takeover());

        let loaded = takeover.load_state();
        assert!(loaded.codex.is_some());

        takeover.save_state(&TakeoverState::default()).unwrap();
        assert!(!takeover.has_stale_takeover());
    }

    #[test]
    fn restore_deletes_files_that_did_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        let takeover = GatewayTakeover::new(dir.path());
        let live = dir.path().join("settings.json");
        fs::write(&live, "placeholder").unwrap();

        let mut state = TakeoverState::default();
        state.claude_code = Some(ClientSnapshot {
            files: vec![SnapshotFile {
                live_path: live.clone(),
                snapshot_name: None,
                private: true,
            }],
        });
        takeover.save_state(&state).unwrap();

        takeover.restore(GatewayClient::ClaudeCode).unwrap();

        assert!(!live.exists());
        assert!(!takeover.has_stale_takeover());
    }

    #[test]
    fn restore_puts_snapshot_content_back() {
        let dir = tempfile::tempdir().unwrap();
        let takeover = GatewayTakeover::new(dir.path());
        fs::create_dir_all(&takeover.snapshot_dir).unwrap();

        let live = dir.path().join("config.toml");
        fs::write(&live, "taken-over").unwrap();
        fs::write(takeover.snapshot_dir.join("codex_config.toml"), "original").unwrap();

        let mut state = TakeoverState::default();
        state.codex = Some(ClientSnapshot {
            files: vec![SnapshotFile {
                live_path: live.clone(),
                snapshot_name: Some("codex_config.toml".to_string()),
                private: false,
            }],
        });
        takeover.save_state(&state).unwrap();

        takeover.restore(GatewayClient::Codex).unwrap();

        assert_eq!(fs::read_to_string(&live).unwrap(), "original");
        assert!(!takeover.has_stale_takeover());
    }
}
