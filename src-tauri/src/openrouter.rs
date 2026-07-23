use crate::{
    models::{OpenRouterModel, OpenRouterStatus},
    storage::NativeResult,
};
use serde::Deserialize;

const SERVICE: &str = "com.dawnlightlabs.edentic";
const ACCOUNT: &str = "openrouter-api-key";

fn entry() -> NativeResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(|error| error.to_string())
}

pub fn has_key() -> bool {
    read_key().is_ok()
}

pub fn save_key(api_key: &str) -> NativeResult<()> {
    if !api_key.starts_with("sk-or-") {
        return Err("This does not look like an OpenRouter API key".into());
    }
    entry()?.set_password(api_key).map_err(|error| error.to_string())
}

pub fn read_key() -> NativeResult<String> {
    entry()?.get_password().map_err(|_| "No OpenRouter API key is configured".into())
}

pub fn delete_key() -> NativeResult<()> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelResponse>,
}

#[derive(Debug, Deserialize)]
struct ModelResponse {
    id: String,
    name: String,
    context_length: Option<u64>,
    pricing: Option<Pricing>,
}

#[derive(Debug, Deserialize)]
struct Pricing {
    prompt: Option<String>,
    completion: Option<String>,
}

fn fetch_models() -> NativeResult<Vec<ModelResponse>> {
    let key = read_key()?;
    let response = reqwest::blocking::Client::new()
        .get("https://openrouter.ai/api/v1/models")
        .bearer_auth(key)
        .header("HTTP-Referer", "https://dawnlightlabs.com")
        .header("X-OpenRouter-Title", "Edentic")
        .send()
        .map_err(|error| format!("Could not reach OpenRouter: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("OpenRouter rejected the request ({})", response.status()));
    }
    response
        .json::<ModelsResponse>()
        .map(|payload| payload.data)
        .map_err(|error| format!("Invalid OpenRouter response: {error}"))
}

pub fn test_connection() -> NativeResult<OpenRouterStatus> {
    let key = read_key()?;
    let response = reqwest::blocking::Client::new()
        .get("https://openrouter.ai/api/v1/key")
        .bearer_auth(key)
        .header("HTTP-Referer", "https://dawnlightlabs.com")
        .header("X-OpenRouter-Title", "Edentic")
        .send()
        .map_err(|error| format!("Could not reach OpenRouter: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("OpenRouter rejected this API key ({})", response.status()));
    }
    let models = fetch_models()?;
    Ok(OpenRouterStatus {
        ok: true,
        message: format!("Connected to OpenRouter · {} models available", models.len()),
        model_count: models.len(),
    })
}

pub fn list_models() -> NativeResult<Vec<OpenRouterModel>> {
    let mut models = fetch_models()?
        .into_iter()
        .map(|model| {
            let is_free = model.id.ends_with(":free")
                || model.id == "openrouter/free"
                || model.pricing.as_ref().is_some_and(|pricing| {
                    pricing.prompt.as_deref() == Some("0") && pricing.completion.as_deref() == Some("0")
                });
            OpenRouterModel {
                id: model.id,
                name: model.name,
                context_length: model.context_length.unwrap_or_default(),
                is_free,
            }
        })
        .collect::<Vec<_>>();
    models.sort_by(|left, right| right.is_free.cmp(&left.is_free).then(left.name.cmp(&right.name)));
    Ok(models)
}
