use yew::prelude::*;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::platform::spawn_local;

use crate::types::LogSession;
use crate::api::{fetch_versions, decode_log_file};
use crate::parser::parse_log_sessions;
use crate::components::SessionView;

#[function_component(App)]
pub fn app(_props: &()) -> Html {
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
                match fetch_versions().await {
                    Ok(v) => {
                        if let Some(first) = v.get(0) {
                            selected_version.set(first.clone());
                        }
                        versions.set(v);
                    },
                    Err(e) => {
                        web_sys::console::log_1(&format!("Error fetching versions: {:?}", e).into());
                    }
                }
            });
            || ()
        });
    }

    let on_version_change = {
        let selected_version = selected_version.clone();
        Callback::from(move |event: Event| {
            let target = event.target_unchecked_into::<HtmlSelectElement>();
            selected_version.set(target.value());
        })
    };

    let on_log_level_change = {
        let log_level = log_level.clone();
        Callback::from(move |event: Event| {
            let target = event.target_unchecked_into::<HtmlSelectElement>();
            log_level.set(target.value());
        })
    };

    let on_file_change = {
        let file = file.clone();
        Callback::from(move |event: Event| {
            let target = event.target_unchecked_into::<HtmlInputElement>();
            let file_obj = target.files().and_then(|list| list.get(0));
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
            spawn_local(async move {
                if let Some(file) = file_opt {
                    match decode_log_file(file, version, log_level).await {
                        Ok(raw) => {
                            web_sys::console::log_1(&raw.clone().into());
                            
                            // Parse the log content into sessions
                            let sessions = parse_log_sessions(&raw);
                            log_sessions.set(sessions);
                        },
                        Err(e) => {
                            web_sys::console::log_1(&format!("Error decoding file: {:?}", e).into());
                            log_sessions.set(vec![LogSession {
                                id: 0,
                                content: format!("Error: {:?}", e),
                                timestamp: None,
                            }]);
                        }
                    }
                } else {
                    log_sessions.set(vec![LogSession {
                        id: 0,
                        content: "No file selected".to_string(),
                        timestamp: None,
                    }]);
                }
            });
        })
    };

    html! {
        <div style="display:flex; flex-direction:row; height:100vh; font-family:Arial,sans-serif;">
            <div style="width:350px; min-width:350px; padding:1.5em; background:#f8f9fa; border-right:1px solid #ddd; display:flex; flex-direction:column; gap:1em;">
                <h1 style="margin:0 0 1em 0; color:#333;">{ "FW Log Decoder" }</h1>
                
                <div style="display:flex; flex-direction:column; gap:0.5em;">
                    <label style="font-weight:bold; color:#555;">{ "Decoder Version:" }</label>
                    <select onchange={on_version_change} style="width:100%; padding:0.5em; border:1px solid #ccc; border-radius:4px;" value={(*selected_version).clone()}>
                        { for versions.iter().map(|version| {
                            html! { <option value={version.clone()}>{ version }</option> }
                        })}
                    </select>
                </div>
                
                <div style="display:flex; flex-direction:column; gap:0.5em;">
                    <label style="font-weight:bold; color:#555;">{ "Log Level:" }</label>
                    <select onchange={on_log_level_change} style="width:100%; padding:0.5em; border:1px solid #ccc; border-radius:4px;" value={(*log_level).clone()}>
                        <option value="0">{ "0 - Critical" }</option>
                        <option value="1">{ "1 - Error" }</option>
                        <option value="2">{ "2 - Warning" }</option>
                        <option value="3">{ "3 - Info" }</option>
                        <option value="4" selected=true>{ "4 - Debug" }</option>
                        <option value="5">{ "5 - Verbose" }</option>
                    </select>
                </div>
                
                <div style="display:flex; flex-direction:column; gap:0.5em;">
                    <label style="font-weight:bold; color:#555;">{ "Log File:" }</label>
                    <input type="file" onchange={on_file_change} style="width:100%; padding:0.5em; border:1px solid #ccc; border-radius:4px;" />
                </div>
                
                <div style="margin-top:1em;">
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
                <SessionView sessions={(*log_sessions).clone()} />
            </div>
        </div>
    }
}
