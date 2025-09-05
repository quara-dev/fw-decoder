use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSession {
    pub id: usize,
    pub content: String,
    pub timestamp: Option<String>,
}

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
    // For backend, we'll use a simpler format
    // This could be enhanced to use proper datetime formatting
    format!("Epoch: {}", epoch)
}

/// Extract timestamp in milliseconds from a log line
/// Expected format: "1234ms\t\t[MODULE]\t\tmessage"
fn extract_timestamp_from_line(line: &str) -> Option<u64> {
    if let Some(ms_pos) = line.find("ms\t") {
        let timestamp_str = &line[..ms_pos];
        timestamp_str.parse::<u64>().ok()
    } else {
        None
    }
}

pub fn parse_log_sessions(log_content: &str) -> Vec<LogSession> {
    let mut sessions = Vec::new();
    let mut current_session = String::new();
    let mut session_id = 0;
    let mut current_session_time: Option<String> = None;
    let mut seen_non_zero_timestamp = false; // Track if we've seen non-zero timestamps in current session
    
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
        
        // Extract timestamp from log line to track boot cycle logic
        let timestamp_ms = extract_timestamp_from_line(line);
        
        // Check for "System Reset Cause" to start a new session
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
                seen_non_zero_timestamp = false; // Reset timestamp tracking
            }
            
            // Add the reset cause line to the new session
            current_session.push_str(&format!("{}\n", line));
        } 
        // Check for boot cycle reset: 0ms after we've seen non-zero timestamps
        else if timestamp_ms == Some(0) && seen_non_zero_timestamp && !current_session.is_empty() {
            // Start new boot cycle - we've seen non-zero timestamps and now hit 0ms again
            sessions.push(LogSession {
                id: session_id,
                content: current_session.trim().to_string(),
                timestamp: current_session_time.clone(),
            });
            session_id += 1;
            current_session.clear();
            current_session_time = None; // Reset for new session
            seen_non_zero_timestamp = false; // Reset timestamp tracking
            
            // Add the 0ms line to the new session
            current_session.push_str(&format!("{}\n", line));
        } else {
            // Add the line to the current session
            current_session.push_str(&format!("{}\n", line));
            
            // Track if we've seen non-zero timestamps
            if let Some(ts) = timestamp_ms {
                if ts > 0 {
                    seen_non_zero_timestamp = true;
                }
            }
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
    
    println!("Parsed {} sessions from log content", sessions.len());
    for (i, session) in sessions.iter().enumerate() {
        println!("Session {}: {} lines, timestamp: {:?}", 
                 i, 
                 session.content.lines().count(),
                 session.timestamp);
    }
    
    sessions
}
