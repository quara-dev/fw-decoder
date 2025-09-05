use syslog_decoder::SyslogParser;
use std::io::Write;
use tempfile::NamedTempFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Comprehensive Syslog Parser Test");
    println!("====================================");
    
    // Create a test dictionary with various formats
    let mut dict_file = NamedTempFile::new()?;
    write!(dict_file, "2;1;boot.c:45;BOOT;System started with %d modules")?;
    write!(dict_file, "\x00")?;
    write!(dict_file, "1;2;main.c:123;APP;Processing task %d")?;
    write!(dict_file, "\x00")?;
    write!(dict_file, "3;3;net.c:67;NETWORK;Received %d bytes from 0x%x at %d")?;
    write!(dict_file, "\x00")?;
    write!(dict_file, "0;4;error.c:89;ERROR;Critical system failure")?;
    write!(dict_file, "\x00")?;
    write!(dict_file, "1;5;debug.c:12;DEBUG;Debug value: %d")?;
    write!(dict_file, "\x00")?;
    dict_file.flush()?;
    
    // Create synthetic binary data with various scenarios
    let mut binary_data = Vec::new();
    
    // Entry 1: Boot message with 2 args (timestamp=0, log_id=0, args=[5])
    binary_data.extend_from_slice(&0u32.to_le_bytes()); // timestamp
    let log_id_1 = (2u32 << 28) | 0u32; // 2 args, offset 0
    binary_data.extend_from_slice(&log_id_1.to_le_bytes());
    binary_data.extend_from_slice(&5u32.to_le_bytes()); // arg1: 5 modules
    binary_data.extend_from_slice(&0u32.to_le_bytes()); // arg2: unused (should show <missing>)
    
    // Entry 2: App processing (timestamp=100, log_id=1, args=[42])
    binary_data.extend_from_slice(&100u32.to_le_bytes()); // timestamp  
    let log_id_2 = (1u32 << 28) | 1u32; // 1 arg, offset 1
    binary_data.extend_from_slice(&log_id_2.to_le_bytes());
    binary_data.extend_from_slice(&42u32.to_le_bytes()); // arg1: task 42
    
    // Entry 3: Network message with 3 args (timestamp=250, log_id=2, args=[1024, 0xDEAD, 123])
    binary_data.extend_from_slice(&250u32.to_le_bytes()); // timestamp
    let log_id_3 = (3u32 << 28) | 2u32; // 3 args, offset 2
    binary_data.extend_from_slice(&log_id_3.to_le_bytes());
    binary_data.extend_from_slice(&1024u32.to_le_bytes()); // arg1: 1024 bytes
    binary_data.extend_from_slice(&0xDEADu32.to_le_bytes()); // arg2: 0xDEAD address
    binary_data.extend_from_slice(&123u32.to_le_bytes()); // arg3: time 123
    
    // Entry 4: Error message with no args (timestamp=500, log_id=3)
    binary_data.extend_from_slice(&500u32.to_le_bytes()); // timestamp
    binary_data.extend_from_slice(&3u32.to_le_bytes()); // log_id (0 args, offset 3)
    
    // Entry 5: Debug message with missing arg (timestamp=1000, log_id=4, only partial args)
    binary_data.extend_from_slice(&1000u32.to_le_bytes()); // timestamp
    let log_id_5 = (1u32 << 28) | 4u32; // 1 arg, offset 4
    binary_data.extend_from_slice(&log_id_5.to_le_bytes());
    binary_data.extend_from_slice(&999u32.to_le_bytes()); // arg1: debug value 999
    
    // Write test binary file
    let temp_binary = NamedTempFile::new()?;
    std::fs::write(temp_binary.path(), binary_data)?;
    
    // Test the parser
    let parser = SyslogParser::new(dict_file.path())?;
    println!("📚 Loaded {} dictionary entries", parser.dictionary_size());
    
    // Test with different log levels
    for level in [0, 2, 4, 6] {
        println!("\n🎯 Testing with log level {}", level);
        let parsed_logs = parser.parse_binary(temp_binary.path(), level)?;
        println!("📊 Parsed {} logs", parsed_logs.len());
        
        let formatted = parser.format_logs(&parsed_logs);
        for (i, log) in formatted.iter().enumerate() {
            println!("  {}: {}", i + 1, log);
        }
    }
    
    // Test edge cases
    println!("\n🧪 Testing edge cases:");
    
    // Test with empty binary file
    let empty_binary = NamedTempFile::new()?;
    std::fs::write(empty_binary.path(), Vec::<u8>::new())?;
    let empty_logs = parser.parse_binary(empty_binary.path(), 5)?;
    println!("📋 Empty file: {} logs", empty_logs.len());
    
    // Test modulo mapping with large offset
    let mut large_offset_data = Vec::new();
    large_offset_data.extend_from_slice(&2000u32.to_le_bytes()); // timestamp
    large_offset_data.extend_from_slice(&1000u32.to_le_bytes()); // large offset (1000 % 5 = 0)
    
    let large_offset_binary = NamedTempFile::new()?;
    std::fs::write(large_offset_binary.path(), large_offset_data)?;
    let modulo_logs = parser.parse_binary(large_offset_binary.path(), 5)?;
    println!("🔄 Modulo mapping: {} logs", modulo_logs.len());
    if !modulo_logs.is_empty() {
        let formatted_modulo = parser.format_logs(&modulo_logs);
        println!("  -> {}", formatted_modulo[0]);
    }
    
    println!("\n✅ All tests completed successfully!");
    
    Ok(())
}
