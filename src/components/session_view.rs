use yew::prelude::*;
use crate::types::LogSession;

#[derive(Properties, PartialEq)]
pub struct SessionViewProps {
    pub sessions: Vec<LogSession>,
}

#[function_component(SessionView)]
pub fn session_view(props: &SessionViewProps) -> Html {
    let sessions = &props.sessions;
    
    if sessions.is_empty() {
        html! {
            <div style="flex:1; display:flex; align-items:center; justify-content:center; color:#888; font-size:1.2em;">
                { "Upload a log file and click Submit to see parsed sessions" }
            </div>
        }
    } else {
        html! {
            <>
                { for sessions.iter().map(|session| {
                    let session_title = if sessions.len() > 1 {
                        if let Some(ref timestamp) = session.timestamp {
                            format!("Session {} (Boot Cycle {}) - {}", session.id + 1, session.id + 1, timestamp)
                        } else {
                            format!("Session {} (Boot Cycle {})", session.id + 1, session.id + 1)
                        }
                    } else {
                        if let Some(ref timestamp) = session.timestamp {
                            format!("Log Output - {}", timestamp)
                        } else {
                            "Log Output".to_string()
                        }
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
    }
}
