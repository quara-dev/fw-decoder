use yew::prelude::*;
use crate::types::LogSession;
use std::collections::HashSet;

fn format_epoch_to_readable(timestamp_str: &str) -> String {
    // Remove "Epoch: " prefix if present
    let clean_timestamp = if timestamp_str.starts_with("Epoch: ") {
        &timestamp_str[7..] // Remove "Epoch: " (7 characters)
    } else {
        timestamp_str
    };
    
    // Try to parse the timestamp as epoch seconds
    if let Ok(epoch_secs) = clean_timestamp.parse::<i64>() {
        // Convert epoch seconds to JavaScript Date
        let epoch_ms = epoch_secs * 1000; // Convert to milliseconds
        
        // Use JavaScript Date for formatting (GMT adjusted)
        let date = js_sys::Date::new(&wasm_bindgen::JsValue::from(epoch_ms as f64));
        
        let day = date.get_utc_date();
        let month = date.get_utc_month() + 1; // JavaScript months are 0-based
        let year = date.get_utc_full_year() % 100; // Get last 2 digits of year
        let hours = date.get_utc_hours();
        let minutes = date.get_utc_minutes();
        let seconds = date.get_utc_seconds();
        
        format!("Date: {:02}/{:02}/{:02} Time: {:02}:{:02}:{:02}", 
               day, month, year, hours, minutes, seconds)
    } else {
        // If parsing fails, return the original timestamp
        timestamp_str.to_string()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LogLevel {
    pub name: String,
    pub color: String,
}

impl LogLevel {
    pub fn from_string(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "CRITICAL" => LogLevel { name: "Critical".to_string(), color: "#dc3545".to_string() },
            "FATALERROR" => LogLevel { name: "FatalError".to_string(), color: "#721c24".to_string() },
            "ERROR" => LogLevel { name: "Error".to_string(), color: "#dc3545".to_string() },
            "WARNING" => LogLevel { name: "Warning".to_string(), color: "#fd7e14".to_string() },
            "INFO" => LogLevel { name: "Info".to_string(), color: "#198754".to_string() },
            "DEBUG" => LogLevel { name: "Debug".to_string(), color: "#6c757d".to_string() },
            "VERBOSE" => LogLevel { name: "Verbose".to_string(), color: "#6f42c1".to_string() },
            _ => LogLevel { name: s.to_string(), color: "#6c757d".to_string() },
        }
    }
}

fn parse_log_levels_from_content(content: &str) -> Vec<LogLevel> {
    let mut levels = HashSet::new();
    for line in content.lines() {
        if let Some(start) = line.find('[') {
            if let Some(end) = line[start..].find(']') {
                let level_part = &line[start+1..start+end];
                // Check if this looks like a log level
                if ["CRITICAL", "FATALERROR", "ERROR", "WARNING", "INFO", "DEBUG", "VERBOSE"]
                    .contains(&level_part.to_uppercase().as_str()) {
                    levels.insert(LogLevel::from_string(level_part));
                }
            }
        }
    }
    levels.into_iter().collect()
}

fn filter_content_by_log_levels(content: &str, enabled_levels: &HashSet<String>, show_log_levels: bool) -> String {
    content.lines()
        .filter(|line| {
            if enabled_levels.is_empty() {
                return true; // Show all if no filter
            }
            
            // Check if line contains any enabled log level
            for level in enabled_levels {
                if line.to_uppercase().contains(&format!("[{}]", level.to_uppercase())) {
                    return true;
                }
            }
            false
        })
        .map(|line| {
            if show_log_levels {
                line.to_string()
            } else {
                // Remove log level from display
                if let Some(start) = line.find('[') {
                    if let Some(end) = line[start..].find(']') {
                        let level_part = &line[start+1..start+end];
                        if ["CRITICAL", "FATALERROR", "ERROR", "WARNING", "INFO", "DEBUG", "VERBOSE"]
                            .contains(&level_part.to_uppercase().as_str()) {
                            let before = &line[..start];
                            let after = &line[start+end+1..];
                            return format!("{}{}", before, after).trim().to_string();
                        }
                    }
                }
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Properties, PartialEq)]
pub struct EnhancedSessionViewProps {
    pub sessions: Vec<LogSession>,
    pub show_log_levels: bool,
}

#[derive(Clone, PartialEq)]
pub struct SessionCategory {
    pub name: String,
    pub sessions: Vec<LogSession>,
}

#[function_component(EnhancedSessionView)]
pub fn enhanced_session_view(props: &EnhancedSessionViewProps) -> Html {
    let sessions = &props.sessions;
    let show_log_levels = props.show_log_levels;
    let selected_session = use_state(|| None::<LogSession>);
    let enabled_log_levels = use_state(|| HashSet::<String>::new());
    
    if sessions.is_empty() {
        return html! {
            <div style="flex:1; display:flex; align-items:center; justify-content:center; color:#888; font-size:1.2em;">
                { "Upload a log file and click Submit to see parsed sessions" }
            </div>
        };
    }
    
    // Categorize sessions
    let mut sessions_with_timestamp = Vec::new();
    let mut sessions_without_timestamp = Vec::new();
    
    for session in sessions.iter() {
        if session.timestamp.is_some() {
            sessions_with_timestamp.push(session.clone());
        } else {
            sessions_without_timestamp.push(session.clone());
        }
    }
    
    let categories = vec![
        SessionCategory {
            name: "Boot Cycles with Timestamp".to_string(),
            sessions: sessions_with_timestamp,
        },
        SessionCategory {
            name: "Boot Cycles without Timestamp".to_string(),
            sessions: sessions_without_timestamp,
        },
    ];
    
    let on_session_click = {
        let selected_session = selected_session.clone();
        let enabled_log_levels = enabled_log_levels.clone();
        Callback::from(move |session: LogSession| {
            // Reset log level filter when opening a new session
            enabled_log_levels.set(HashSet::new());
            selected_session.set(Some(session));
        })
    };
    
    let on_modal_close = {
        let selected_session = selected_session.clone();
        Callback::from(move |_| {
            selected_session.set(None);
        })
    };

    html! {
        <>
            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 2em; height: 100%;">
                { for categories.iter().filter(|cat| !cat.sessions.is_empty()).map(|category| {
                    html! {
                        <div style="display: flex; flex-direction: column; border: 1px solid #ddd; border-radius: 8px; overflow: hidden; background: #f9f9f9;">
                            <div style="background: #4a5568; color: white; padding: 1em; text-align: center; font-weight: bold; font-size: 1.1em;">
                                { &category.name }
                                <div style="font-size: 0.9em; font-weight: normal; margin-top: 0.25em; opacity: 0.8;">
                                    { format!("{} session{}", category.sessions.len(), if category.sessions.len() != 1 { "s" } else { "" }) }
                                </div>
                            </div>
                            <div style="flex: 1; padding: 1em; display: flex; flex-direction: column; gap: 0.75em; overflow-y: auto; max-height: calc(100vh - 300px);">
                                { for category.sessions.iter().enumerate().map(|(index, session)| {
                                    let session_clone = session.clone();
                                    let on_click = {
                                        let session = session_clone.clone();
                                        let callback = on_session_click.clone();
                                        Callback::from(move |_: MouseEvent| {
                                            callback.emit(session.clone());
                                        })
                                    };
                                    
                                    let session_title = if let Some(ref timestamp) = session.timestamp {
                                        format!("Session {} - {}", index + 1, format_epoch_to_readable(timestamp))
                                    } else {
                                        format!("Session {}", index + 1)
                                    };
                                    
                                    let preview_lines: Vec<&str> = session.content.lines().take(3).collect();
                                    let preview_text = if preview_lines.len() > 0 {
                                        let preview = preview_lines.join("\n");
                                        if session.content.lines().count() > 3 {
                                            format!("{}...", preview)
                                        } else {
                                            preview
                                        }
                                    } else {
                                        "Empty session".to_string()
                                    };
                                    
                                    html! {
                                        <div 
                                            onclick={on_click}
                                            style="
                                                border: 1px solid #ccc; 
                                                border-radius: 6px; 
                                                padding: 0.75em; 
                                                cursor: pointer; 
                                                background: white; 
                                                transition: all 0.2s ease;
                                            "
                                            class="session-card"
                                        >
                                            <div style="font-weight: bold; color: #333; margin-bottom: 0.5em; font-size: 0.95em;">
                                                { session_title }
                                            </div>
                                            <div style="
                                                font-family: 'Courier New', monospace; 
                                                font-size: 0.8em; 
                                                color: #666; 
                                                white-space: pre-line; 
                                                line-height: 1.3;
                                                overflow: hidden;
                                                text-overflow: ellipsis;
                                                display: -webkit-box;
                                                -webkit-line-clamp: 3;
                                                -webkit-box-orient: vertical;
                                            ">
                                                { if show_log_levels {
                                                    preview_text
                                                } else {
                                                    filter_content_by_log_levels(&preview_text, &HashSet::new(), false)
                                                }}
                                            </div>
                                            <div style="margin-top: 0.5em; font-size: 0.75em; color: #888;">
                                                { format!("{} lines", session.content.lines().count()) }
                                            </div>
                                        </div>
                                    }
                                }) }
                            </div>
                        </div>
                    }
                }) }
            </div>

            { if let Some(ref session) = *selected_session {
                let session_title = if let Some(ref timestamp) = session.timestamp {
                    format!("Session Details - {}", format_epoch_to_readable(timestamp))
                } else {
                    "Session Details".to_string()
                };

                // Get all available log levels from this session
                let available_levels = parse_log_levels_from_content(&session.content);
                
                // Apply log level filtering and display preferences
                let filtered_content = filter_content_by_log_levels(
                    &session.content, 
                    &*enabled_log_levels, 
                    show_log_levels
                );

                html! {
                    <div 
                        style="
                            position: fixed; 
                            top: 0; 
                            left: 0; 
                            width: 100%; 
                            height: 100%; 
                            background: rgba(0,0,0,0.5); 
                            display: flex; 
                            align-items: center; 
                            justify-content: center; 
                            z-index: 1000;
                        "
                        onclick={on_modal_close.clone()}
                    >
                        <div 
                            style="
                                background: white; 
                                width: 90%; 
                                height: 90%; 
                                border-radius: 8px; 
                                display: flex; 
                                flex-direction: column; 
                                overflow: hidden;
                                box-shadow: 0 10px 25px rgba(0,0,0,0.2);
                            "
                            onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}
                        >
                            <div style="
                                background: #4a5568; 
                                color: white; 
                                padding: 1em 1.5em; 
                                display: flex; 
                                justify-content: space-between; 
                                align-items: center;
                            ">
                                <h3 style="margin: 0; font-size: 1.2em;">{ session_title }</h3>
                                <button 
                                    onclick={on_modal_close.clone()}
                                    style="
                                        background: none; 
                                        border: none; 
                                        color: white; 
                                        font-size: 1.5em; 
                                        cursor: pointer; 
                                        padding: 0; 
                                        width: 2em; 
                                        height: 2em; 
                                        display: flex; 
                                        align-items: center; 
                                        justify-content: center;
                                        border-radius: 4px;
                                    "
                                    class="close-button"
                                >
                                    { "×" }
                                </button>
                            </div>
                            
                            { if !available_levels.is_empty() {
                                let enabled_log_levels_clone = enabled_log_levels.clone();
                                html! {
                                    <div style="
                                        background: #f8f9fa; 
                                        border-bottom: 1px solid #dee2e6; 
                                        padding: 1em 1.5em;
                                        display: flex;
                                        flex-wrap: wrap;
                                        gap: 0.5em;
                                        align-items: center;
                                    ">
                                        <strong style="margin-right: 1em; color: #495057;">{ "Filter by log level:" }</strong>
                                        { for available_levels.iter().map(|level| {
                                            let level_name = level.name.clone();
                                            let is_enabled = enabled_log_levels.contains(&level_name);
                                            let enabled_log_levels_for_click = enabled_log_levels_clone.clone();
                                            
                                            let onclick = Callback::from(move |_: MouseEvent| {
                                                let mut current = (*enabled_log_levels_for_click).clone();
                                                if current.contains(&level_name) {
                                                    current.remove(&level_name);
                                                } else {
                                                    current.insert(level_name.clone());
                                                }
                                                enabled_log_levels_for_click.set(current);
                                            });

                                            html! {
                                                <button
                                                    onclick={onclick}
                                                    style={format!(
                                                        "
                                                        background: {}; 
                                                        color: white; 
                                                        border: none; 
                                                        padding: 0.25em 0.75em; 
                                                        border-radius: 20px; 
                                                        cursor: pointer; 
                                                        font-size: 0.8em;
                                                        opacity: {};
                                                        transition: opacity 0.2s;
                                                        ",
                                                        level.color,
                                                        if is_enabled { "1" } else { "0.5" }
                                                    )}
                                                >
                                                    { &level.name }
                                                </button>
                                            }
                                        }) }
                                        <button
                                            onclick={
                                                let enabled_log_levels = enabled_log_levels.clone();
                                                Callback::from(move |_: MouseEvent| {
                                                    enabled_log_levels.set(HashSet::new());
                                                })
                                            }
                                            style="
                                                background: #6c757d; 
                                                color: white; 
                                                border: none; 
                                                padding: 0.25em 0.75em; 
                                                border-radius: 20px; 
                                                cursor: pointer; 
                                                font-size: 0.8em;
                                                margin-left: 1em;
                                            "
                                        >
                                            { "Show All" }
                                        </button>
                                    </div>
                                }
                            } else {
                                html! {}
                            }}

                            <div style="flex: 1; overflow: hidden; display: flex; flex-direction: column;">
                                <textarea 
                                    readonly=true
                                    value={filtered_content}
                                    style="
                                        flex: 1; 
                                        font-family: 'Courier New', monospace; 
                                        font-size: 0.9em; 
                                        padding: 1.5em; 
                                        border: none; 
                                        outline: none; 
                                        resize: none; 
                                        line-height: 1.4; 
                                        background: #f8f9fa;
                                    "
                                />
                            </div>
                        </div>
                    </div>
                }
            } else {
                html! {}
            }}
        </>
    }
}
