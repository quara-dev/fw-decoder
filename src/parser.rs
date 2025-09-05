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
