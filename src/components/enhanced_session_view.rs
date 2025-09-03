use yew::prelude::*;
use crate::types::LogSession;
use web_sys::HtmlElement;

#[derive(Properties, PartialEq)]
pub struct EnhancedSessionViewProps {
    pub sessions: Vec<LogSession>,
}

#[derive(Clone, PartialEq)]
pub struct SessionCategory {
    pub name: String,
    pub sessions: Vec<LogSession>,
}

#[function_component(EnhancedSessionView)]
pub fn enhanced_session_view(props: &EnhancedSessionViewProps) -> Html {
    let sessions = &props.sessions;
    let selected_session = use_state(|| None::<LogSession>);
    
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
        Callback::from(move |session: LogSession| {
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
                                        format!("Session {} - {}", index + 1, timestamp)
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
                                                border: 1px solid #cbd5e0; 
                                                border-radius: 6px; 
                                                padding: 1em; 
                                                background: white; 
                                                cursor: pointer; 
                                                transition: all 0.2s ease;
                                            "
                                            class="session-card"
                                        >
                                            <div style="font-weight: bold; color: #2d3748; margin-bottom: 0.5em; font-size: 0.95em;">
                                                { session_title }
                                            </div>
                                            <div style="font-family: monospace; font-size: 0.8em; color: #666; line-height: 1.3; white-space: pre-wrap; overflow: hidden;">
                                                { preview_text }
                                            </div>
                                            <div style="margin-top: 0.5em; font-size: 0.75em; color: #999; text-align: right;">
                                                { format!("{} lines", session.content.lines().count()) }
                                            </div>
                                        </div>
                                    }
                                })}
                            </div>
                        </div>
                    }
                })}
            </div>
            
            // Modal for displaying full session content
            { if let Some(ref session) = *selected_session {
                let session_title = if let Some(ref timestamp) = session.timestamp {
                    format!("Session {} - {}", session.id + 1, timestamp)
                } else {
                    format!("Session {}", session.id + 1)
                };
                
                html! {
                    <div 
                        style="
                            position: fixed; 
                            top: 0; 
                            left: 0; 
                            width: 100vw; 
                            height: 100vh; 
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
                                border-radius: 8px; 
                                width: 90vw; 
                                height: 80vh; 
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
                            <div style="flex: 1; overflow: hidden; display: flex; flex-direction: column;">
                                <textarea 
                                    readonly=true
                                    value={session.content.clone()}
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
