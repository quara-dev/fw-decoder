use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::prelude::*;
use crate::types::LogSession;

pub async fn fetch_versions() -> Result<Vec<String>, JsValue> {
    let window = web_sys::window().ok_or("window not available")?;
    let resp_value = JsFuture::from(window.fetch_with_str("/api/versions")).await?;
    let resp: web_sys::Response = resp_value.dyn_into()?;
    let json = JsFuture::from(resp.json()?).await?;
    let arr = js_sys::Array::from(&json);
    let mut versions = Vec::new();
    for i in 0..arr.length() {
        versions.push(arr.get(i).as_string().unwrap_or_default());
    }
    Ok(versions)
}

pub async fn decode_log_file_with_options(file: web_sys::File, version: String, log_level: String, _include_log_level: bool) -> Result<Vec<LogSession>, JsValue> {
    let form = web_sys::FormData::new()?;
    form.append_with_blob("file", &file)?;
    
    // Always request log levels from backend - frontend will control display
    let url = format!("/api/decode?version={}&log_level={}&include_log_level=true", 
                     version, log_level);
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&form.into());
    
    let request = web_sys::Request::new_with_str_and_init(&url, &opts)?;
    let window = web_sys::window().ok_or("window not available")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: web_sys::Response = resp_value.dyn_into()?;
    let json = JsFuture::from(resp.json()?).await?;
    
    // Parse the JSON response as sessions
    let sessions: Vec<LogSession> = serde_wasm_bindgen::from_value(json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse sessions: {}", e)))?;
    
    Ok(sessions)
}
