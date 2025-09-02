use crate::types::LogSession;

pub fn parse_date_time_line(line: &str) -> Option<u64> {
    // Parse both formats:
    // "Date time set rcvd: 1756474625" (standalone)
    // "69808ms		[SYS_PROTOCOL_DATE_TIME]	Date time set rcvd: 1756474625" (with timestamp and module)
    
    if line.contains("Date time set rcvd:") {
        // Find the part after "Date time set rcvd:"
        if let Some(start_pos) = line.find("Date time set rcvd:") {
            let after_colon = &line[start_pos + "Date time set rcvd:".len()..];
            let timestamp_str = after_colon.trim();
            if let Ok(epoch) = timestamp_str.parse::<u64>() {
                return Some(epoch);
            }
        }
    }
    None
}

pub fn epoch_to_local_time(epoch: u64) -> String {
    // Convert epoch timestamp to human-readable format
    web_sys::console::log_1(&format!("Converting epoch: {}", epoch).into());
    
    // Create JavaScript Date object with epoch time in milliseconds
    let timestamp_ms = (epoch as f64) * 1000.0;
    web_sys::console::log_1(&format!("Timestamp in ms: {}", timestamp_ms).into());
    
    let js_date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(timestamp_ms));
    
    // Use basic toString() method
    match js_date.to_string().as_string() {
        Some(date_str) => {
            web_sys::console::log_1(&format!("Formatted result: {}", date_str).into());
            date_str
        },
        None => {
            let fallback = format!("Epoch: {}", epoch);
            web_sys::console::log_1(&format!("Fallback result: {}", fallback).into());
            fallback
        }
    }
}

pub fn parse_log_line(line: &str) -> Option<(u64, String, String)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    
    // Split by tabs first
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() >= 3 {
        // Format: "timestamp\t\t[module]\tcontent"
        let timestamp_part = parts[0];
        let module_part = parts[2];
        let content_part = if parts.len() > 3 {
            parts[3..].join("\t")
        } else {
            String::new()
        };
        
        // Extract timestamp (remove "ms" suffix)
        if let Some(timestamp_str) = timestamp_part.strip_suffix("ms") {
            if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                // Extract module name (remove brackets)
                let module = module_part.trim_start_matches('[').trim_end_matches(']').to_string();
                return Some((timestamp, module, content_part));
            }
        }
    }
    
    None
}

pub fn parse_log_sessions(log_content: &str) -> Vec<LogSession> {
    let mut sessions = Vec::new();
    let mut current_session = String::new();
    let mut session_id = 0;
    let mut last_timestamp: Option<u64> = None;
    let mut found_first_timestamp = false;
    let mut current_session_time: Option<String> = None;
    
    for line in log_content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        // Check for "Date time set rcvd" line to extract epoch timestamp
        if let Some(epoch_time) = parse_date_time_line(line) {
            current_session_time = Some(epoch_to_local_time(epoch_time));
            current_session.push_str(&format!("{}\n", line));
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
                                timestamp: current_session_time.clone(),
                            });
                            session_id += 1;
                            current_session.clear();
                            current_session_time = None; // Reset for new session
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
            timestamp: current_session_time,
        });
    }
    
    // If no sessions were created, treat entire content as one session
    if sessions.is_empty() && !log_content.trim().is_empty() {
        sessions.push(LogSession {
            id: 0,
            content: log_content.to_string(),
            timestamp: None,
        });
    }
    
    sessions
}
