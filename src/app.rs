use yew::prelude::*;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::platform::spawn_local;

use crate::types::LogSession;
use crate::api::{fetch_versions, decode_log_file_with_options};
use crate::components::EnhancedSessionView;

#[derive(Clone, PartialEq)]
pub enum ProcessingState {
    Idle,
    Loading,
    Success,
    Error(String),
}

#[function_component(App)]
pub fn app(_props: &()) -> Html {
    let versions = use_state(|| Vec::<String>::new());
    let selected_version = use_state(|| String::new());
    let log_level = use_state(|| "4".to_string());
    let include_log_level = use_state(|| false);
    let log_sessions = use_state(|| Vec::<LogSession>::new());
    let file = use_state(|| None);
    let processing_state = use_state(|| ProcessingState::Idle);
    let progress_message = use_state(|| String::new());

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

    let on_include_log_level_change = {
        let include_log_level = include_log_level.clone();
        Callback::from(move |event: Event| {
            let target = event.target_unchecked_into::<HtmlInputElement>();
            include_log_level.set(target.checked());
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
        let include_log_level = include_log_level.clone();
        let file = file.clone();
        let log_sessions = log_sessions.clone();
        let processing_state = processing_state.clone();
        let progress_message = progress_message.clone();
        Callback::from(move |_| {
            let version = (*selected_version).clone();
            let log_level = (*log_level).clone();
            let include_log_level = *include_log_level;
            let file_opt = (*file).clone();
            let log_sessions = log_sessions.clone();
            let processing_state = processing_state.clone();
            let progress_message = progress_message.clone();
            
            if file_opt.is_none() {
                processing_state.set(ProcessingState::Error("No file selected".to_string()));
                return;
            }
            
            // Set loading state immediately
            processing_state.set(ProcessingState::Loading);
            progress_message.set("Uploading file and starting decoding process...".to_string());
            
            spawn_local(async move {
                if let Some(file) = file_opt {
                    // Update progress message
                    progress_message.set(format!("Processing file: {} (this may take a while for large files)", file.name()));
                    
                    match decode_log_file_with_options(file, version, log_level, include_log_level).await {
                        Ok(sessions) => {
                            progress_message.set("Processing completed successfully!".to_string());
                            
                            if sessions.is_empty() {
                                processing_state.set(ProcessingState::Error("Decoder returned no sessions. File may be invalid or log level too restrictive.".to_string()));
                                progress_message.set("No sessions found".to_string());
                                log_sessions.set(vec![LogSession {
                                    id: 0,
                                    content: "No sessions found. The file may be invalid, corrupted, or the log level filter may be too restrictive.".to_string(),
                                    timestamp: None,
                                }]);
                            } else {
                                log_sessions.set(sessions.clone());
                                processing_state.set(ProcessingState::Success);
                                progress_message.set(format!("Processing completed successfully! Found {} sessions", sessions.len()));
                            }
                        },
                        Err(e) => {
                            let error_msg = format!("Error decoding file: {:?}", e);
                            web_sys::console::log_1(&error_msg.clone().into());
                            processing_state.set(ProcessingState::Error(error_msg.clone()));
                            progress_message.set(error_msg);
                            log_sessions.set(vec![LogSession {
                                id: 0,
                                content: format!("Error: {:?}", e),
                                timestamp: None,
                            }]);
                        }
                    }
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
                
                <div style="display:flex; align-items:center; gap:0.5em;">
                    <input 
                        type="checkbox" 
                        id="include-log-level"
                        onchange={on_include_log_level_change} 
                        checked={*include_log_level}
                    />
                    <label for="include-log-level" style="color:#555; cursor:pointer;">
                        { "Include log levels in output (Emergency, Alert, Critical, etc.)" }
                    </label>
                </div>
                
                <div style="margin-top:1em;">
                    <button 
                        onclick={on_submit} 
                        disabled={matches!(*processing_state, ProcessingState::Loading)}
                        style={format!(
                            "width:100%;padding:0.7em 0; font-size:1em; {}",
                            if matches!(*processing_state, ProcessingState::Loading) {
                                "background:#ccc; cursor:not-allowed;"
                            } else {
                                "background:#007bff; color:white; cursor:pointer;"
                            }
                        )}
                    >
                        { match &*processing_state {
                            ProcessingState::Loading => "Processing...",
                            _ => "Decode Log"
                        }}
                    </button>
                </div>
                
                { match &*processing_state {
                    ProcessingState::Loading => html! {
                        <div style="margin-top:1em; padding:1em; background:#e7f3ff; border:1px solid #b3d9ff; border-radius:4px;">
                            <div style="display:flex; align-items:center; gap:0.5em;">
                                <div class="spinner" style="
                                    width:16px; height:16px; 
                                    border:2px solid #f3f3f3; 
                                    border-top:2px solid #007bff; 
                                    border-radius:50%; 
                                    animation:spin 1s linear infinite;
                                "></div>
                                <strong style="color:#0056b3;">{ "Processing..." }</strong>
                            </div>
                            <div style="margin-top:0.5em; color:#0056b3; font-size:0.9em;">
                                { &*progress_message }
                            </div>
                            <div style="margin-top:0.5em; color:#666; font-size:0.8em;">
                                { "Please wait while the executable processes your file. This may take several minutes for large files." }
                            </div>
                        </div>
                    },
                    ProcessingState::Success => html! {
                        <div style="margin-top:1em; padding:1em; background:#d4edda; border:1px solid #c3e6cb; border-radius:4px;">
                            <strong style="color:#155724;">{ "✓ Success!" }</strong>
                            <div style="margin-top:0.5em; color:#155724; font-size:0.9em;">
                                { &*progress_message }
                            </div>
                        </div>
                    },
                    ProcessingState::Error(msg) => html! {
                        <div style="margin-top:1em; padding:1em; background:#f8d7da; border:1px solid #f5c6cb; border-radius:4px;">
                            <strong style="color:#721c24;">{ "✗ Error!" }</strong>
                            <div style="margin-top:0.5em; color:#721c24; font-size:0.9em;">
                                { msg }
                            </div>
                        </div>
                    },
                    ProcessingState::Idle => html! {}
                }}
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
                <EnhancedSessionView sessions={(*log_sessions).clone()} />
            </div>
        </div>
    }
}
