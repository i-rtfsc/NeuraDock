//! Gateway takeover: snapshot a tool's live config, point it at the local
//! gateway with a process-local credential, and restore the snapshot afterwards.
//!
//! The real relay key never reaches the tool's config file while the gateway
//! is on, and stopping the gateway (or
//! the next app start after a crash) puts the original file back exactly.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use super::{ClaudeConfigService, CodexConfigService};
use neuradock_infrastructure::fs_safe::atomic_write;
use neuradock_infrastructure::gateway::GatewayClient;

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

struct LiveFileState {
    path: PathBuf,
    content: Option<String>,
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

    /// Whether the selected client's live config is currently owned by the
    /// running gateway. This is read-only and never changes takeover state.
    pub fn is_active_for(&self, client: GatewayClient) -> bool {
        self.client_snapshot(&self.load_state(), client).is_some()
    }

    /// Take over `client`'s tool config: snapshot the live files, then write a
    /// config pointing at `gateway_base_url` with the current local credential.
    pub fn take_over(
        &self,
        client: GatewayClient,
        gateway_base_url: &str,
        local_credential: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<()> {
        let mut state = self.load_state();
        if self.client_snapshot(&state, client).is_some() {
            // Already taken over: just refresh the managed config so a
            // new port or binding lands in the tool config.
            self.write_gateway_config(
                client,
                gateway_base_url,
                local_credential,
                model,
                context_1m,
            )?;
            return Ok(());
        }

        let snapshot = self.snapshot_files(client)?;
        *self.client_snapshot_mut(&mut state, client) = Some(snapshot);
        self.save_state(&state)?;

        if let Err(write_error) = self.write_gateway_config(
            client,
            gateway_base_url,
            local_credential,
            model,
            context_1m,
        ) {
            if let Err(restore_error) = self.restore(client) {
                return Err(write_error.context(format!(
                    "Gateway config rollback also failed: {restore_error}"
                )));
            }
            return Err(write_error);
        }
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
        let live_state = self.capture_snapshot_targets(&snapshot)?;
        let restore_result = (|| -> Result<()> {
            for file in &snapshot.files {
                match &file.snapshot_name {
                    Some(name) => {
                        let content = fs::read_to_string(self.snapshot_dir.join(name))
                            .with_context(|| format!("Failed to read snapshot {}", name))?;
                        // A truncated or empty snapshot must abort, not be
                        // written back. `read_to_string` happily returns Ok("")
                        // for a zero-length file, so without this check a
                        // snapshot that lost its data would be restored over
                        // the user's real config -- turning a crash window into
                        // permanent credential loss. Returning Err here routes
                        // into the rollback below, which puts the live files
                        // back and leaves the takeover state intact so the
                        // startup self-heal can retry.
                        validate_snapshot_content(name, &content)?;
                        atomic_write(&file.live_path, &content, file.private)?;
                    }
                    None => match fs::remove_file(&file.live_path) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(error.into()),
                    },
                }
            }
            Ok(())
        })();
        if let Err(error) = restore_result {
            self.restore_live_files(&live_state)
                .context("Failed to roll back an incomplete gateway restore")?;
            return Err(error);
        }

        self.save_state(&state)?;
        log::info!("Gateway takeover restored for {}", client.display_name());
        Ok(())
    }

    /// Forget `client`'s takeover record **without** writing the snapshot back.
    ///
    /// Used when the live config has deliberately been replaced by something
    /// the user wants to keep -- switching an agent to direct write, where the
    /// real key has already been written to the tool config by the time this
    /// runs. Calling `restore` there would overwrite that fresh config with the
    /// pre-takeover snapshot; leaving the record in place is just as bad,
    /// because `restore_all` on exit would then silently revert the user's
    /// direct config on every shutdown.
    pub fn discard(&self, client: GatewayClient) -> Result<()> {
        let mut state = self.load_state();
        if self
            .client_snapshot_mut(&mut state, client)
            .take()
            .is_none()
        {
            return Ok(());
        }
        self.save_state(&state)?;
        log::info!(
            "Gateway takeover record discarded for {} (config intentionally replaced)",
            client.display_name()
        );
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
                fs::copy(&live_path, self.snapshot_dir.join(name))
                    .with_context(|| format!("Failed to snapshot {}", live_path.display()))?;
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

    fn write_gateway_config(
        &self,
        client: GatewayClient,
        gateway_base_url: &str,
        local_credential: &str,
        model: Option<&str>,
        context_1m: bool,
    ) -> Result<()> {
        let live_state = self.capture_live_files(client)?;
        let result = match client {
            GatewayClient::Codex => CodexConfigService::new().configure_global_with_key(
                local_credential,
                gateway_base_url,
                model,
                context_1m,
            ),
            GatewayClient::ClaudeCode => ClaudeConfigService::new().configure_global_with_key(
                local_credential,
                gateway_base_url,
                model,
                context_1m,
            ),
        };
        if let Err(error) = result {
            self.restore_live_files(&live_state)
                .with_context(|| format!("Failed to roll back {} config", client.display_name()))?;
            return Err(error);
        }
        Ok(())
    }

    fn capture_live_files(&self, client: GatewayClient) -> Result<Vec<LiveFileState>> {
        let targets = match client {
            GatewayClient::Codex => {
                let (config, auth) = CodexConfigService::managed_paths()?;
                vec![(config, false), (auth, true)]
            }
            GatewayClient::ClaudeCode => {
                vec![(ClaudeConfigService::managed_path()?, true)]
            }
        };
        self.capture_targets(targets)
    }

    fn capture_snapshot_targets(&self, snapshot: &ClientSnapshot) -> Result<Vec<LiveFileState>> {
        self.capture_targets(
            snapshot
                .files
                .iter()
                .map(|file| (file.live_path.clone(), file.private))
                .collect(),
        )
    }

    fn capture_targets(&self, targets: Vec<(PathBuf, bool)>) -> Result<Vec<LiveFileState>> {
        targets
            .into_iter()
            .map(|(path, private)| {
                let content = if path.exists() {
                    Some(
                        fs::read_to_string(&path)
                            .with_context(|| format!("Failed to read {}", path.display()))?,
                    )
                } else {
                    None
                };
                Ok(LiveFileState {
                    path,
                    content,
                    private,
                })
            })
            .collect()
    }

    fn restore_live_files(&self, files: &[LiveFileState]) -> Result<()> {
        for file in files {
            match &file.content {
                Some(content) => atomic_write(&file.path, content, file.private)?,
                None => match fs::remove_file(&file.path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                },
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
        atomic_write(
            &self.state_path,
            &serde_json::to_string_pretty(state)?,
            false,
        )?;
        Ok(())
    }
}

/// Reject a snapshot that cannot be the user's original config.
///
/// Restoring is the last line of defence for credentials, so a snapshot that
/// is empty or structurally invalid is treated as corruption rather than
/// content. Writing such a file back would overwrite a live config with
/// nothing, which is unrecoverable; failing instead preserves both the live
/// files and the takeover record so the operation can be retried.
fn validate_snapshot_content(name: &str, content: &str) -> Result<()> {
    if content.trim().is_empty() {
        anyhow::bail!("Snapshot {name} is empty; refusing to overwrite the live config with it");
    }

    if name.ends_with(".json") {
        serde_json::from_str::<serde_json::Value>(content)
            .with_context(|| format!("Snapshot {name} is not valid JSON"))?;
    } else if name.ends_with(".toml") {
        content
            .parse::<toml_edit::DocumentMut>()
            .with_context(|| format!("Snapshot {name} is not valid TOML"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_snapshot_is_rejected() {
        assert!(validate_snapshot_content("claude_settings.json", "").is_err());
        assert!(validate_snapshot_content("claude_settings.json", "   \n").is_err());
    }

    #[test]
    fn malformed_snapshot_is_rejected_by_format() {
        assert!(validate_snapshot_content("codex_auth.json", "{\"a\":").is_err());
        assert!(validate_snapshot_content("codex_config.toml", "a = [[[").is_err());
    }

    #[test]
    fn well_formed_snapshots_pass() {
        validate_snapshot_content("codex_auth.json", r#"{"OPENAI_API_KEY":"sk-x"}"#).unwrap();
        validate_snapshot_content("codex_config.toml", "model_provider = \"x\"\n").unwrap();
        // Unknown extensions only get the emptiness check.
        validate_snapshot_content("config.snapshot", "anything").unwrap();
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    // The snapshot/restore mechanics are exercised against temp dirs; the
    // placeholder write itself touches the real ~/.codex and is covered by
    // the config service tests, so these tests focus on state bookkeeping.

    #[test]
    fn state_round_trips_and_clears_when_empty() {
        let dir = tempfile::tempdir().unwrap();
        let takeover = GatewayTakeover::new(dir.path());

        let state = TakeoverState {
            codex: Some(ClientSnapshot {
                files: vec![SnapshotFile {
                    live_path: dir.path().join("config.toml"),
                    snapshot_name: Some("codex_config.toml".to_string()),
                    private: false,
                }],
            }),
            ..Default::default()
        };
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

        let state = TakeoverState {
            claude_code: Some(ClientSnapshot {
                files: vec![SnapshotFile {
                    live_path: live.clone(),
                    snapshot_name: None,
                    private: true,
                }],
            }),
            ..Default::default()
        };
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
        fs::write(&live, "model_provider = \"gateway\"\n").unwrap();
        // Content must be valid TOML: restore refuses to write back a snapshot
        // it cannot parse, so a placeholder string would exercise the
        // corruption guard rather than the round-trip this test covers.
        fs::write(
            takeover.snapshot_dir.join("codex_config.toml"),
            "model_provider = \"original\"\n",
        )
        .unwrap();

        let state = TakeoverState {
            codex: Some(ClientSnapshot {
                files: vec![SnapshotFile {
                    live_path: live.clone(),
                    snapshot_name: Some("codex_config.toml".to_string()),
                    private: false,
                }],
            }),
            ..Default::default()
        };
        takeover.save_state(&state).unwrap();

        takeover.restore(GatewayClient::Codex).unwrap();

        assert_eq!(
            fs::read_to_string(&live).unwrap(),
            "model_provider = \"original\"\n"
        );
        assert!(!takeover.has_stale_takeover());
    }

    /// Switching to direct write must drop the takeover record while leaving
    /// the freshly written config alone. Restoring instead would clobber the
    /// user's real key; keeping the record would make exit revert it.
    #[test]
    fn discard_clears_state_without_touching_live_config() {
        let dir = tempfile::tempdir().unwrap();
        let takeover = GatewayTakeover::new(dir.path());
        fs::create_dir_all(&takeover.snapshot_dir).unwrap();

        let live = dir.path().join("config.toml");
        // What directConfigure() just wrote: the user's real relay key.
        fs::write(&live, "model_provider = \"user-direct\"\n").unwrap();
        fs::write(
            takeover.snapshot_dir.join("codex_config.toml"),
            "model_provider = \"pre-takeover\"\n",
        )
        .unwrap();

        let state = TakeoverState {
            codex: Some(ClientSnapshot {
                files: vec![SnapshotFile {
                    live_path: live.clone(),
                    snapshot_name: Some("codex_config.toml".to_string()),
                    private: false,
                }],
            }),
            ..Default::default()
        };
        takeover.save_state(&state).unwrap();

        takeover.discard(GatewayClient::Codex).unwrap();

        assert_eq!(
            fs::read_to_string(&live).unwrap(),
            "model_provider = \"user-direct\"\n",
            "discard must not roll the live config back"
        );
        assert!(
            !takeover.has_stale_takeover(),
            "record must be gone so exit does not revert the direct config"
        );
        // Discarding again is a no-op, not an error.
        takeover.discard(GatewayClient::Codex).unwrap();
    }

    /// A snapshot that lost its contents (crash between write and fsync) must
    /// abort the restore, leave the live file untouched, and keep the takeover
    /// record so the startup self-heal can retry.
    #[test]
    fn empty_snapshot_aborts_restore_and_preserves_state() {
        let dir = tempfile::tempdir().unwrap();
        let takeover = GatewayTakeover::new(dir.path());
        fs::create_dir_all(&takeover.snapshot_dir).unwrap();

        let live = dir.path().join("config.toml");
        fs::write(&live, "model_provider = \"gateway\"\n").unwrap();
        fs::write(takeover.snapshot_dir.join("codex_config.toml"), "").unwrap();

        let state = TakeoverState {
            codex: Some(ClientSnapshot {
                files: vec![SnapshotFile {
                    live_path: live.clone(),
                    snapshot_name: Some("codex_config.toml".to_string()),
                    private: false,
                }],
            }),
            ..Default::default()
        };
        takeover.save_state(&state).unwrap();

        assert!(takeover.restore(GatewayClient::Codex).is_err());
        assert_eq!(
            fs::read_to_string(&live).unwrap(),
            "model_provider = \"gateway\"\n",
            "live config must not be clobbered by an empty snapshot"
        );
        assert!(
            takeover.has_stale_takeover(),
            "state must survive so the restore can be retried"
        );
    }

    #[test]
    fn incomplete_restore_rolls_live_files_back_and_keeps_state() {
        let dir = tempfile::tempdir().unwrap();
        let takeover = GatewayTakeover::new(dir.path());
        fs::create_dir_all(&takeover.snapshot_dir).unwrap();

        let config = dir.path().join("config.toml");
        let auth = dir.path().join("auth.json");
        fs::write(&config, "gateway-config").unwrap();
        fs::write(&auth, "gateway-auth").unwrap();
        fs::write(
            takeover.snapshot_dir.join("config.snapshot"),
            "original-config",
        )
        .unwrap();

        let state = TakeoverState {
            codex: Some(ClientSnapshot {
                files: vec![
                    SnapshotFile {
                        live_path: config.clone(),
                        snapshot_name: Some("config.snapshot".to_string()),
                        private: false,
                    },
                    SnapshotFile {
                        live_path: auth.clone(),
                        snapshot_name: Some("missing.snapshot".to_string()),
                        private: true,
                    },
                ],
            }),
            ..Default::default()
        };
        takeover.save_state(&state).unwrap();

        assert!(takeover.restore(GatewayClient::Codex).is_err());
        assert_eq!(fs::read_to_string(config).unwrap(), "gateway-config");
        assert_eq!(fs::read_to_string(auth).unwrap(), "gateway-auth");
        assert!(takeover.has_stale_takeover());
    }
}
