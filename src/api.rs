use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::prelude::*;

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

pub async fn decode_log_file(file: web_sys::File, version: String, log_level: String) -> Result<String, JsValue> {
    let form = web_sys::FormData::new()?;
    form.append_with_blob("file", &file)?;
    
    let url = format!("/api/decode?version={}&log_level={}", version, log_level);
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&form.into());
    
    let request = web_sys::Request::new_with_str_and_init(&url, &opts)?;
    let window = web_sys::window().ok_or("window not available")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: web_sys::Response = resp_value.dyn_into()?;
    let text = JsFuture::from(resp.text()?).await?;
    let raw = text.as_string().unwrap_or_else(|| format!("{:?}", text));
    
    Ok(raw)
}
