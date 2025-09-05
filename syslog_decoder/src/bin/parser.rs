use syslog_decoder::SyslogParser;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 4 {
        eprintln!("Usage: {} <dictionary.log> <binary.bin> <log_level>", args[0]);
        eprintln!("Example: {} Quara_fw_9.17.3.0.log syslog_9_17_3_0_F344.bin 5", args[0]);
        std::process::exit(1);
    }
    
    let dict_path = &args[1];
    let binary_path = &args[2]; 
    let log_level: u8 = args[3].parse()?;
    
    println!("Syslog Parser v0.1.0");
    println!("Dictionary: {}", dict_path);
    println!("Binary: {}", binary_path);
    println!("Log level: {}", log_level);
    println!("---");
    
    // Create parser
    let parser = SyslogParser::new(dict_path)?;
    println!("Loaded {} dictionary entries", parser.dictionary_size());
    
    // Parse binary file
    let parsed_logs = parser.parse_binary(binary_path, log_level)?;
    println!("Parsed {} log entries", parsed_logs.len());
    
    // Format and output logs
    let formatted_logs = parser.format_logs(&parsed_logs);
    for log in formatted_logs {
        println!("{}", log);
    }
    
    Ok(())
}
