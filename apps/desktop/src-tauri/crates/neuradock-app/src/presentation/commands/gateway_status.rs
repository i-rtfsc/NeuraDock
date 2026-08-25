use super::gateway::{
    GatewayBindingDto, GatewayClientDto, GatewayStatusDto, GatewayUpstreamStatusDto,
};
use super::gateway_state::GatewayState;

pub(super) async fn status_of(state: &GatewayState) -> GatewayStatusDto {
    let (running, base_url) = {
        let server = state.server.lock().await;
        (
            server.is_some(),
            server.as_ref().map(|handle| handle.base_url()),
        )
    };
    let meta = state.meta.lock().await.clone();
    let upstreams = state.bindings.read().await.clone();
    let mut bindings = Vec::new();
    for (route, meta) in meta {
        let client = route.client;
        let upstream = if let Some(configured) = upstreams.get(&route) {
            let snapshot = state.runtime.snapshot(client, configured).await;
            Some(GatewayUpstreamStatusDto {
                id: snapshot.id,
                label: snapshot.label,
                circuit_state: snapshot.state.into(),
                consecutive_failures: snapshot.consecutive_failures,
                total_requests: snapshot.total_requests,
                successful_requests: snapshot.successful_requests,
                failed_requests: snapshot.failed_requests,
                input_tokens: snapshot.usage.input_tokens,
                output_tokens: snapshot.usage.output_tokens,
                cache_read_tokens: snapshot.usage.cache_read_tokens,
                cache_creation_tokens: snapshot.usage.cache_creation_tokens,
                average_latency_ms: snapshot.average_latency_ms,
                average_first_event_latency_ms: snapshot.average_first_event_latency_ms,
                last_error: snapshot.last_error,
                last_error_at_ms: snapshot.last_error_at_ms,
                recovery_at_ms: snapshot.recovery_at_ms,
            })
        } else {
            None
        };
        bindings.push(GatewayBindingDto {
            client: client.into(),
            profile: route.profile,
            label: meta.label,
            model: meta.model,
            context1m: meta.context1m,
            upstream_kind: meta.kind,
            impersonate: meta.impersonate,
            upstream,
        });
    }
    bindings.sort_by_key(|binding| binding.client == GatewayClientDto::ClaudeCode);
    GatewayStatusDto {
        running,
        base_url,
        bindings,
    }
}
