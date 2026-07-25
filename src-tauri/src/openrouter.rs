use crate::{
    models::{OpenRouterModel, OpenRouterStatus},
    storage::NativeResult,
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    io::{BufRead, BufReader},
    time::Duration,
};

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
    entry()?
        .set_password(api_key)
        .map_err(|error| format!("Windows Credential Manager could not store the key: {error}"))?;
    let restored = read_key()
        .map_err(|error| format!("The key was written but could not be read back: {error}"))?;
    if restored != api_key {
        return Err("Windows Credential Manager returned a different credential after saving".into());
    }
    Ok(())
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
    #[serde(default)]
    architecture: Architecture,
    #[serde(default)]
    supported_parameters: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Architecture {
    #[serde(default)]
    input_modalities: Vec<String>,
    #[serde(default)]
    output_modalities: Vec<String>,
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
                input_modalities: model.architecture.input_modalities,
                output_modalities: model.architecture.output_modalities,
                supported_parameters: model.supported_parameters,
            }
        })
        .collect::<Vec<_>>();
    models.sort_by(|left, right| right.is_free.cmp(&left.is_free).then(left.name.cmp(&right.name)));
    Ok(models)
}

pub fn validate_model_capabilities(model_id: &str, requires_images: bool) -> NativeResult<()> {
    if model_id == "openrouter/free" {
        return Ok(());
    }
    let model = fetch_models()?.into_iter().find(|model| model.id == model_id)
        .ok_or_else(|| format!("The configured OpenRouter model '{model_id}' is unavailable"))?;
    if requires_images && !model.architecture.input_modalities.iter().any(|value| value == "image") {
        return Err(format!(
            "The configured model '{}' cannot inspect images. Choose a vision-capable model in Settings.",
            model.name
        ));
    }
    if !model.supported_parameters.iter().any(|value|
        value == "response_format" || value == "structured_outputs"
    ) {
        return Err(format!(
            "The configured model '{}' does not advertise structured outputs. Choose a model with response_format support.",
            model.name
        ));
    }
    Ok(())
}

pub fn stream_structured_response<Cancelled, Chunk>(
    model: &str,
    messages: Value,
    schema_name: &str,
    schema: Value,
    mut cancelled: Cancelled,
    mut on_chunk: Chunk,
) -> NativeResult<String>
where
    Cancelled: FnMut() -> bool,
    Chunk: FnMut(&str),
{
    let request = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "temperature": 0.2,
        "provider": {"allow_fallbacks": true, "require_parameters": true},
        "response_format": {
            "type": "json_schema",
            "json_schema": {"name": schema_name, "strict": true, "schema": schema}
        }
    });
    let response = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(360))
        .build().map_err(|error| error.to_string())?
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(read_key()?)
        .header("HTTP-Referer", "https://dawnlightlabs.com")
        .header("X-OpenRouter-Title", "Edentic")
        .json(&request)
        .send().map_err(|error| format!("Could not reach OpenRouter: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let detail = response.text().unwrap_or_default().chars().take(800).collect::<String>();
        return Err(format!("OpenRouter rejected the request ({status}): {detail}"));
    }
    let mut result = String::new();
    for line in BufReader::new(response).lines() {
        if cancelled() {
            return Err("Remote request cancelled".into());
        }
        let line = line.map_err(|error| format!("OpenRouter stream failed: {error}"))?;
        let Some(data) = line.strip_prefix("data:") else { continue };
        let data = data.trim();
        if data == "[DONE]" { break; }
        let Ok(event) = serde_json::from_str::<Value>(data) else { continue };
        if let Some(error) = event.get("error") {
            return Err(format!("OpenRouter stream error: {error}"));
        }
        if let Some(content) = event.pointer("/choices/0/delta/content").and_then(Value::as_str) {
            result.push_str(content);
            on_chunk(content);
        }
    }
    if cancelled() { return Err("Remote request cancelled".into()); }
    if result.trim().is_empty() { return Err("OpenRouter returned no structured response".into()); }
    Ok(result)
}
