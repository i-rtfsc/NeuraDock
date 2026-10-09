use crate::presentation::{
    error::CommandError,
    state::{Repositories, Services},
};
use neuradock_domain::independent_key::IndependentKeyId;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn fetch_independent_key_models(
    key_id: i64,
    repositories: State<'_, Repositories>,
    services: State<'_, Services>,
) -> Result<Vec<String>, CommandError> {
    let key = repositories
        .independent_key
        .find_by_id(&IndependentKeyId::new(key_id))
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::not_found("Key not found"))?;
    if !key.is_active() {
        return Err(CommandError::validation("The selected key is inactive"));
    }
    let base = key.base_url().trim_end_matches('/');
    let url = if base.ends_with("/v1") {
        format!("{base}/models")
    } else {
        format!("{base}/v1/models")
    };
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none());
    if let Some(proxy) = services
        .proxy_config
        .get()
        .await
        .ok()
        .and_then(|config| config.proxy_url())
    {
        builder = builder.proxy(
            reqwest::Proxy::all(proxy)
                .map_err(|error| CommandError::infrastructure(error.to_string()))?,
        );
    }
    let client = builder
        .build()
        .map_err(|error| CommandError::infrastructure(error.to_string()))?;
    let response = client
        .get(url)
        .bearer_auth(key.api_key())
        .header("x-api-key", key.api_key())
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|error| CommandError::infrastructure(error.to_string()))?
        .error_for_status()
        .map_err(|error| CommandError::infrastructure(error.to_string()))?;
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|error| CommandError::infrastructure(error.to_string()))?;
    let mut models: Vec<String> = body
        .get("data")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| CommandError::validation("The model endpoint did not return a model list"))?
        .iter()
        .filter_map(|model| model.get("id")?.as_str().map(str::to_string))
        .collect();
    models.sort();
    models.dedup();
    Ok(models)
}
