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

pub fn format_timestamp_ms(timestamp_ms: u64) -> String {
    let total_seconds = timestamp_ms / 1000;
    let milliseconds = timestamp_ms % 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    
    if minutes > 0 {
        format!("{:02}:{:02}.{:03}", minutes, seconds, milliseconds)
    } else {
        format!("{:02}.{:03}", seconds, milliseconds)
    }
}

pub fn parse_log_line(line: &str) -> Option<(u64, String, String)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    
    // Look for pattern: "timestampms   [module] content" with potential spaces
    // First try to extract timestamp with "ms" suffix
    if let Some(ms_pos) = line.find("ms") {
        let timestamp_part = &line[..ms_pos];
        
        if let Ok(timestamp) = timestamp_part.parse::<u64>() {
            let rest = &line[ms_pos + 2..]; // Get everything after "ms"
            
            // Look for module in square brackets, skipping any whitespace
            if let Some(bracket_start) = rest.find('[') {
                if let Some(bracket_end) = rest.find(']') {
                    let module = rest[bracket_start + 1..bracket_end].to_string();
                    let content = rest[bracket_end + 1..].trim().to_string();
                    return Some((timestamp, module, content));
                }
            }
        }
    }
    
    None
}

pub fn parse_log_sessions(log_content: &str) -> Vec<LogSession> {
    let mut sessions = Vec::new();
    let mut current_session = String::new();
    let mut session_id = 0;
    let mut current_session_time: Option<String> = None;
    
    for line in log_content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        // Skip decoder messages that shouldn't be displayed
        if line.contains("Using default dictionnay") || 
           line.contains("Using default dictionary") ||
           line.starts_with("Using default") {
            continue;
        }
        
        // Check for "Date time set rcvd" line to extract epoch timestamp
        if let Some(epoch_time) = parse_date_time_line(line) {
            current_session_time = Some(epoch_to_local_time(epoch_time));
            current_session.push_str(&format!("{}\n", line));
            continue;
        }
        
        // Check for "System Reset Cause" FIRST before timestamp parsing
        if line.contains("System Reset Cause") {
            // If we have content in current session, save it before starting new one
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
            
            // Try to parse and format this "System Reset Cause" line
            if let Some((timestamp, module, content)) = parse_log_line(line) {
                let formatted_timestamp = format_timestamp_ms(timestamp);
                current_session.push_str(&format!("{}\t\t[{}]\t{}\n", formatted_timestamp, module, content));
            } else {
                // Add as-is if parsing fails
                current_session.push_str(&format!("{}\n", line));
            }
        }
        // Try to parse and reformat other lines with timestamp formatting
        else if let Some((timestamp, module, content)) = parse_log_line(line) {
            let formatted_timestamp = format_timestamp_ms(timestamp);
            current_session.push_str(&format!("{}\t\t[{}]\t{}\n", formatted_timestamp, module, content));
        } else {
            // Add the line as-is if it doesn't match the expected format
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
    
    // Filter out sessions with only one line (likely not useful boot sessions)
    sessions.retain(|session| {
        let line_count = session.content.lines().filter(|line| !line.trim().is_empty()).count();
        line_count > 1
    });
    
    // Re-assign session IDs after filtering
    for (index, session) in sessions.iter_mut().enumerate() {
        session.id = index;
    }
    
    // If no sessions were created, treat entire content as one session (but only if it has multiple lines)
    if sessions.is_empty() && !log_content.trim().is_empty() {
        let line_count = log_content.lines().filter(|line| !line.trim().is_empty()).count();
        if line_count > 1 {
            sessions.push(LogSession {
                id: 0,
                content: log_content.to_string(),
                timestamp: None,
            });
        }
    }
    
    sessions
}
