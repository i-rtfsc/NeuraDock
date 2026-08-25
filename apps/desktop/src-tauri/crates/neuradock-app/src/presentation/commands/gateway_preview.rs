use super::gateway::AgentConfigPreviewFileDto;
use super::gateway::GatewayClientDto;
use super::gateway::RawPreviewFile;
use super::gateway_fingerprint::{claude_fingerprint, codex_fingerprint, AgentConfigFingerprint};
use super::gateway_redaction::redact_content;
use crate::presentation::error::CommandError;

pub(super) fn fingerprint_from_preview(
    client: GatewayClientDto,
    files: &[RawPreviewFile],
) -> Result<AgentConfigFingerprint, CommandError> {
    match client {
        GatewayClientDto::ClaudeCode => {
            let (path, _, _, next) = files
                .first()
                .ok_or_else(|| CommandError::infrastructure("Claude preview is empty"))?;
            claude_fingerprint(next, path.clone()).map_err(CommandError::from)
        }
        GatewayClientDto::Codex => {
            let config = files
                .iter()
                .find(|(_, format, _, _)| *format == "toml")
                .ok_or_else(|| CommandError::infrastructure("Codex config preview is missing"))?;
            let auth = files.iter().find(|(_, format, _, _)| *format == "json");
            let auth_content = auth.map(|file| file.3.as_str()).unwrap_or("");
            let paths = files.iter().map(|file| file.0.clone()).collect();
            codex_fingerprint(&config.3, auth_content, paths).map_err(CommandError::from)
        }
    }
}

pub(super) fn preview_files_to_dto(
    raw: Vec<RawPreviewFile>,
    gateway_method: bool,
    gateway_running: bool,
) -> Vec<AgentConfigPreviewFileDto> {
    raw.into_iter()
        .map(|(path, format, current, mut next)| {
            if gateway_method && !gateway_running {
                next = current.clone();
            }
            AgentConfigPreviewFileDto {
                path: path.display().to_string(),
                format: format.to_string(),
                current_content: redact_content(format, &current),
                next_content: redact_content(format, &next),
            }
        })
        .collect()
}
