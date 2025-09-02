use wasm_bindgen::prelude::*;
use yew::prelude::*;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use wasm_bindgen_futures::JsFuture;
use yew::platform::spawn_local;

#[derive(Clone, PartialEq)]
pub struct LogSession {
    pub id: usize,
    pub content: String,
}

fn parse_log_sessions(log_content: &str) -> Vec<LogSession> {
    let mut sessions = Vec::new();
    let mut current_session = String::new();
    let mut session_id = 0;
    let mut last_timestamp: Option<u64> = None;
    let mut found_first_timestamp = false;
    
    for line in log_content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        // Parse line format: "timestamp[ms] [module] content"
        if let Some(parsed) = parse_log_line(line) {
            let (timestamp, module, content) = parsed;
            
            // Check if timestamp has reset (indicating new boot cycle)
            if found_first_timestamp {
                if let Some(last_ts) = last_timestamp {
                    // Detect session boundary: timestamp reset to 0 or very low value after high value
                    if timestamp == 0 && last_ts > 1000 {
                        // New session detected - save current session and start new one
                        if !current_session.is_empty() {
                            sessions.push(LogSession {
                                id: session_id,
                                content: current_session.trim().to_string(),
                            });
                            session_id += 1;
                            current_session.clear();
                        }
                    }
                }
            } else {
                found_first_timestamp = true;
            }
            
            // Add formatted line to current session
            current_session.push_str(&format!("{}ms\t\t[{}]\t{}\n", timestamp, module, content));
            last_timestamp = Some(timestamp);
        } else {
            // If line doesn't match format, add it as-is (like "Using default dictionnay")
            current_session.push_str(&format!("{}\n", line));
        }
    }
    
    // Add the last session
    if !current_session.is_empty() {
        sessions.push(LogSession {
            id: session_id,
            content: current_session.trim().to_string(),
        });
    }
    
    // If no sessions were created, treat entire content as one session
    if sessions.is_empty() && !log_content.trim().is_empty() {
        sessions.push(LogSession {
            id: 0,
            content: log_content.to_string(),
        });
    }
    
    sessions
}

fn parse_log_line(line: &str) -> Option<(u64, String, String)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    
    // Look for timestamp with "ms" suffix
    let timestamp_end = if let Some(pos) = line.find("ms") {
        pos
    } else {
        return None;
    };
    
    let timestamp_str = &line[..timestamp_end];
    let rest = &line[timestamp_end + 2..].trim();
    
    // Parse timestamp (could be decimal or hex)
    let timestamp = if timestamp_str.starts_with("0x") {
        u64::from_str_radix(&timestamp_str[2..], 16).ok()?
    } else {
        timestamp_str.parse().ok()?
    };
    
    // Find module name in square brackets
    if let Some(start) = rest.find('[') {
        if let Some(end) = rest.find(']') {
            if end > start {
                let module = rest[start + 1..end].to_string();
                let content = if end + 1 < rest.len() {
                    rest[end + 1..].trim().to_string()
                } else {
                    String::new()
                };
                return Some((timestamp, module, content));
            }
        }
    }
    
    None
}

#[function_component]
pub fn App(_props: &()) -> Html {
    let versions = use_state(|| Vec::<String>::new());
    let selected_version = use_state(|| String::new());
    let log_level = use_state(|| "4".to_string());
    let log_sessions = use_state(|| Vec::<LogSession>::new());
    let file = use_state(|| None);

    // Fetch versions from backend on mount
    {
        let versions = versions.clone();
        let selected_version = selected_version.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                let window = web_sys::window().unwrap();
                let resp_value = JsFuture::from(window.fetch_with_str("http://localhost:8080/api/versions")).await.unwrap();
                let resp: web_sys::Response = resp_value.dyn_into().unwrap();
                let json = JsFuture::from(resp.json().unwrap()).await.unwrap();
                let arr = js_sys::Array::from(&json);
                let mut v = Vec::new();
                for i in 0..arr.length() {
                    v.push(arr.get(i).as_string().unwrap_or_default());
                }
                if let Some(first) = v.get(0) {
                    selected_version.set(first.clone());
                }
                versions.set(v);
            });
            || ()
        });
    }

    let on_version_change = {
        let selected_version = selected_version.clone();
        Callback::from(move |e: Event| {
            let select = e.target_dyn_into::<HtmlSelectElement>().unwrap();
            selected_version.set(select.value());
        })
    };

    let on_log_level_change = {
        let log_level = log_level.clone();
        Callback::from(move |e: Event| {
            let select = e.target_dyn_into::<HtmlSelectElement>().unwrap();
            log_level.set(select.value());
        })
    };

    let on_file_upload = {
        let file = file.clone();
        Callback::from(move |e: Event| {
            let input = e.target_dyn_into::<HtmlInputElement>().unwrap();
            let file_obj = input.files().and_then(|fs| fs.get(0));
            file.set(file_obj);
        })
    };

    let on_submit = {
        let selected_version = selected_version.clone();
        let log_level = log_level.clone();
        let file = file.clone();
        let log_sessions = log_sessions.clone();
        Callback::from(move |_| {
            let version = (*selected_version).clone();
            let log_level = (*log_level).clone();
            let file_opt = (*file).clone();
            let log_sessions = log_sessions.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(file) = file_opt {
                    let form = web_sys::FormData::new().unwrap();
                    form.append_with_blob("file", &file).unwrap();
                    let url = format!("http://localhost:8080/api/decode?version={}&log_level={}", version, log_level);
                    let opts = web_sys::RequestInit::new();
                    opts.set_method("POST");
                    opts.set_body(&form.into());
                    let request = web_sys::Request::new_with_str_and_init(&url, &opts).unwrap();
                    let window = web_sys::window().unwrap();
                    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await.unwrap();
                    let resp: web_sys::Response = resp_value.dyn_into().unwrap();
                    let text = wasm_bindgen_futures::JsFuture::from(resp.text().unwrap()).await.unwrap();
                    let raw = text.as_string().unwrap_or_else(|| format!("{:?}", text));
                    web_sys::console::log_1(&raw.clone().into());
                    
                    // Parse the log content into sessions
                    let sessions = parse_log_sessions(&raw);
                    log_sessions.set(sessions);
                } else {
                    log_sessions.set(vec![LogSession {
                        id: 0,
                        content: "No file selected".to_string(),
                    }]);
                }
            });
        })
    };

    html! {
        <div style="display:flex; height:100vh;">
            <div style="width:320px; min-width:220px; background:#f7f7f7; padding:2em; box-sizing:border-box; border-right:1px solid #ddd;">
                <h1 style="font-size:1.3em;">{ "FW Log Decoder" }</h1>
                <div style="margin-top:2em;">
                    <label>{ "Version: " }
                        <select id="version-select" onchange={on_version_change} style="width:100%;">
                            { for versions.iter().map(|v| html! { <option value={v.clone()}>{ v }</option> }) }
                        </select>
                    </label>
                </div>
                <div style="margin-top:2em;">
                    <label>{ "Upload File: " }
                        <input type="file" id="file-input" onchange={on_file_upload} style="width:100%;" />
                    </label>
                </div>
                <div style="margin-top:2em;">
                    <label>{ "Log Level: " }
                        <select id="loglevel-select" onchange={on_log_level_change} style="width:100%;">
                            <option value="1">{ "Fatal Error" }</option>
                            <option value="2">{ "Error" }</option>
                            <option value="3">{ "Warn" }</option>
                            <option value="4" selected=true>{ "Info" }</option>
                            <option value="5">{ "Debug" }</option>
                            <option value="6">{ "Trace" }</option>
                        </select>
                    </label>
                </div>
                <div style="margin-top:2em;">
                    <button onclick={on_submit} style="width:100%;padding:0.7em 0; font-size:1em;">{ "Submit" }</button>
                </div>
                { if !log_sessions.is_empty() {
                    html! {
                        <div style="margin-top:2em;">
                            <strong>{ format!("Sessions Found: {}", log_sessions.len()) }</strong>
                        </div>
                    }
                } else {
                    html! {}
                }}
            </div>
            <div style="flex:1; display:flex; flex-direction:column; padding:1em; gap:1em; overflow-y:auto;">
                { if log_sessions.is_empty() {
                    html! {
                        <div style="flex:1; display:flex; align-items:center; justify-content:center; color:#888; font-size:1.2em;">
                            { "Upload a log file and click Submit to see parsed sessions" }
                        </div>
                    }
                } else {
                    html! {
                        <>
                            { for log_sessions.iter().map(|session| {
                                let session_title = if log_sessions.len() > 1 {
                                    format!("Session {} (Boot Cycle {})", session.id + 1, session.id + 1)
                                } else {
                                    "Log Output".to_string()
                                };
                                
                                html! {
                                    <div style="display:flex; flex-direction:column; min-height:200px; border:1px solid #ddd; border-radius:4px; overflow:hidden;">
                                        <div style="background:#f5f5f5; padding:0.5em 1em; border-bottom:1px solid #ddd; font-weight:bold;">
                                            { session_title }
                                        </div>
                                        <textarea 
                                            rows=15 
                                            style="flex:1; width:100%; min-height:200px; resize:vertical; font-family:monospace; font-size:0.9em; padding:1em; box-sizing:border-box; border:none; background:#fff;" 
                                            readonly=true 
                                            placeholder="No log content in this session..."
                                            value={session.content.clone()} 
                                        />
                                    </div>
                                }
                            })}
                        </>
                    }
                }}
            </div>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("root").unwrap();
    yew::Renderer::<App>::with_root(root).render();
}
