use super::agent_access::{AgentAccessSourceDto, AgentIdDto};
use super::gateway::{GatewayClientDto, GatewaySourceDto, UpstreamKindDto};
use crate::application::services::GatewayBindingSourceInput;
use neuradock_infrastructure::gateway::{GatewayClient, GatewayRoute};

pub(super) fn binding_route(client: GatewayClient, isolated_slug: Option<&str>) -> GatewayRoute {
    GatewayRoute {
        client,
        profile: isolated_slug.unwrap_or("default").to_string(),
    }
}

pub(super) fn upstream_kind_str(kind: UpstreamKindDto) -> String {
    match kind {
        UpstreamKindDto::OpenaiChat => "openai_chat",
        UpstreamKindDto::OpenaiResponses => "openai_responses",
        UpstreamKindDto::Anthropic => "anthropic",
    }
    .to_string()
}

pub(super) fn access_source_of(source: &GatewaySourceDto) -> AgentAccessSourceDto {
    match source {
        GatewaySourceDto::AccountToken {
            account_id,
            provider_id,
            token_id,
            base_url,
        } => AgentAccessSourceDto::AccountToken {
            account_id: account_id.clone(),
            provider_id: provider_id.clone(),
            token_id: *token_id,
            base_url: base_url.clone(),
        },
        GatewaySourceDto::IndependentKey { key_id } => {
            AgentAccessSourceDto::IndependentKey { key_id: *key_id }
        }
    }
}

pub(super) fn source_from_access(source: &AgentAccessSourceDto) -> GatewaySourceDto {
    match source {
        AgentAccessSourceDto::AccountToken {
            account_id,
            provider_id,
            token_id,
            base_url,
        } => GatewaySourceDto::AccountToken {
            account_id: account_id.clone(),
            provider_id: provider_id.clone(),
            token_id: *token_id,
            base_url: base_url.clone(),
        },
        AgentAccessSourceDto::IndependentKey { key_id } => {
            GatewaySourceDto::IndependentKey { key_id: *key_id }
        }
    }
}

pub(super) fn client_for_agent(agent: AgentIdDto) -> GatewayClientDto {
    match agent {
        AgentIdDto::Codex => GatewayClientDto::Codex,
        AgentIdDto::ClaudeCode => GatewayClientDto::ClaudeCode,
    }
}

pub(super) fn binding_source_input(source: &GatewaySourceDto) -> GatewayBindingSourceInput {
    match source {
        GatewaySourceDto::AccountToken {
            account_id,
            provider_id,
            token_id,
            base_url,
        } => GatewayBindingSourceInput::AccountToken {
            account_id: account_id.clone(),
            provider_id: provider_id.clone(),
            token_id: *token_id,
            base_url: base_url.clone(),
        },
        GatewaySourceDto::IndependentKey { key_id } => {
            GatewayBindingSourceInput::IndependentKey { key_id: *key_id }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_route_defaults_to_standard_profile() {
        assert_eq!(
            binding_route(GatewayClient::Codex, None),
            GatewayRoute::default(GatewayClient::Codex)
        );
    }

    #[test]
    fn binding_route_preserves_isolated_profile() {
        let route = binding_route(GatewayClient::ClaudeCode, Some("relay-a"));

        assert_eq!(route.client, GatewayClient::ClaudeCode);
        assert_eq!(route.profile, "relay-a");
    }
}
