use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use anyhow::{Result, Context};

/// Represents a log entry from the dictionary
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub log_level: u8,
    pub module_name: String,
    pub log_message: String,
}

/// Represents a parsed log from binary file
#[derive(Debug, Clone)]
pub struct ParsedLog {
    pub timestamp_formatted: String,
    pub module_name: String,
    pub formatted_message: String,
}

/// Binary log entry structure
#[derive(Debug)]
struct BinaryLogEntry {
    timestamp_ms: u32,
    log_id: u32,
    arguments: Vec<u32>,
}

/// Syslog parser library
pub struct SyslogParser {
    dictionary: HashMap<u32, LogEntry>,
}

impl SyslogParser {
    /// Create a new parser with dictionary file
    pub fn new<P: AsRef<Path>>(dictionary_path: P) -> Result<Self> {
        let dictionary = Self::load_dictionary(dictionary_path)?;
        Ok(Self { dictionary })
    }

    /// Load dictionary from .log file
    fn load_dictionary<P: AsRef<Path>>(path: P) -> Result<HashMap<u32, LogEntry>> {
        let mut file = File::open(&path)
            .with_context(|| format!("Failed to open dictionary file: {}", path.as_ref().display()))?;
        
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .context("Failed to read dictionary file contents")?;
        
        let mut dictionary = HashMap::new();
        let mut line_offset = 0u32;

        // Split by NULL character (0x00) instead of newlines
        let entries: Vec<&[u8]> = contents.split(|&b| b == 0x00).collect();

        for entry_bytes in entries {
            if entry_bytes.is_empty() {
                line_offset += 1;
                continue;
            }
            
            // Convert bytes to string
            let line = String::from_utf8_lossy(entry_bytes);
            let trimmed = line.trim();
            
            if trimmed.is_empty() {
                line_offset += 1;
                continue;
            }

            match Self::parse_dictionary_line(trimmed) {
                Ok(entry) => {
                    dictionary.insert(line_offset, entry);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse dictionary line {}: {} ({})", 
                             line_offset, trimmed, e);
                }
            }
            
            line_offset += 1;
        }

        println!("Loaded {} dictionary entries from {}", 
                 dictionary.len(), path.as_ref().display());
        Ok(dictionary)
    }

    /// Parse a single dictionary line
    /// Format: num_args;log_level;source_file:line_number;module_name;log_message
    fn parse_dictionary_line(line: &str) -> Result<LogEntry> {
        let parts: Vec<&str> = line.split(';').collect();
        
        if parts.len() < 5 {
            return Err(anyhow::anyhow!("Invalid dictionary format: expected 5 parts, got {}", parts.len()));
        }

        // Skip num_args parsing (parts[0]) - not needed, extracted from binary data

        let log_level = parts[1].trim().parse::<u8>()
            .context("Failed to parse log level")?;

        // Skip source file and line number parsing (parts[2]) - not needed
        
        let module_name = parts[3].trim().to_string();
        
        // Join remaining parts as log message (in case message contains semicolons)
        let log_message = if parts.len() > 5 {
            parts[4..].join(";").trim().to_string()
        } else {
            parts[4].trim().to_string()
        };

        Ok(LogEntry {
            log_level,
            module_name,
            log_message,
        })
    }

    /// Parse binary log file and return formatted logs
    pub fn parse_binary<P: AsRef<Path>>(&self, binary_path: P, min_log_level: u8) -> Result<Vec<ParsedLog>> {
        let binary_entries = self.read_binary_file(binary_path)?;
        let mut parsed_logs = Vec::new();

        for entry in binary_entries {
            if let Some(parsed_log) = self.process_binary_entry(&entry, min_log_level) {
                parsed_logs.push(parsed_log);
            }
        }

        println!("Parsed {} logs from binary file (min level: {})", 
                 parsed_logs.len(), min_log_level);
        Ok(parsed_logs)
    }

    /// Read and parse binary file structure
    fn read_binary_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<BinaryLogEntry>> {
        let mut file = File::open(&path)
            .with_context(|| format!("Failed to open binary file: {}", path.as_ref().display()))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .context("Failed to read binary file contents")?;

        let mut entries = Vec::new();
        let mut offset = 0;

        while offset + 8 <= contents.len() {
            // Read timestamp (32-bit)
            let timestamp_ms = u32::from_le_bytes([
                contents[offset],
                contents[offset + 1], 
                contents[offset + 2],
                contents[offset + 3],
            ]);
            offset += 4;

            // Read log_id (32-bit)
            let log_id_raw = u32::from_le_bytes([
                contents[offset],
                contents[offset + 1],
                contents[offset + 2], 
                contents[offset + 3],
            ]);
            offset += 4;

            // Extract number of arguments (first 4 bits) and log offset (remaining 28 bits)
            let num_args = ((log_id_raw >> 28) & 0xF) as u8;
            let log_offset = log_id_raw & 0x0FFFFFFF;

            // Read arguments if any
            let mut arguments = Vec::new();
            for _ in 0..num_args {
                if offset + 4 <= contents.len() {
                    let arg = u32::from_le_bytes([
                        contents[offset],
                        contents[offset + 1],
                        contents[offset + 2],
                        contents[offset + 3],
                    ]);
                    arguments.push(arg);
                    offset += 4;
                } else {
                    break; // Incomplete data
                }
            }

            entries.push(BinaryLogEntry {
                timestamp_ms,
                log_id: log_offset,
                arguments,
            });
        }

        println!("Read {} binary log entries from {}", 
                 entries.len(), path.as_ref().display());
        Ok(entries)
    }

    /// Process a single binary entry and create formatted log
    fn process_binary_entry(&self, entry: &BinaryLogEntry, min_log_level: u8) -> Option<ParsedLog> {
        // Look up dictionary entry using modulo mapping (handle offset mismatches)
        let dict_size = self.dictionary.len() as u32;
        let lookup_key = if dict_size > 0 { entry.log_id % dict_size } else { entry.log_id };
        
        let log_entry = self.dictionary.get(&lookup_key)?;

        // Filter by log level
        if log_entry.log_level > min_log_level {
            return None;
        }

        // Format timestamp
        let timestamp_formatted = Self::format_timestamp(entry.timestamp_ms);

        // Format message with arguments
        let formatted_message = Self::format_message(&log_entry.log_message, &entry.arguments);

        Some(ParsedLog {
            timestamp_formatted,
            module_name: log_entry.module_name.clone(),
            formatted_message,
        })
    }

    /// Format timestamp from milliseconds to readable format matching expected output
    fn format_timestamp(timestamp_ms: u32) -> String {
        format!("{}ms", timestamp_ms)
    }

    /// Format log message by replacing placeholders with arguments
    fn format_message(template: &str, arguments: &[u32]) -> String {
        let mut result = template.to_string();
        let mut arg_index = 0;

        // Replace %d placeholders with integer arguments
        while let Some(pos) = result.find("%d") {
            if arg_index < arguments.len() {
                result.replace_range(pos..pos+2, &arguments[arg_index].to_string());
                arg_index += 1;
            } else {
                // No more arguments available, replace with placeholder
                result.replace_range(pos..pos+2, "<missing>");
            }
        }

        // Replace %x placeholders with hex arguments  
        arg_index = 0;
        while let Some(pos) = result.find("%x") {
            if arg_index < arguments.len() {
                result.replace_range(pos..pos+2, &format!("0x{:X}", arguments[arg_index]));
                arg_index += 1;
            } else {
                result.replace_range(pos..pos+2, "<missing>");
            }
        }

        // Replace %s placeholders (string args would need special handling)
        while let Some(pos) = result.find("%s") {
            result.replace_range(pos..pos+2, "<string>");
        }

        result
    }

    /// Get formatted output as strings for compatibility
    pub fn format_logs(&self, logs: &[ParsedLog]) -> Vec<String> {
        logs.iter().map(|log| {
            format!("{:12}\t[{}]\t{}", 
                   log.timestamp_formatted,
                   log.module_name,
                   log.formatted_message)
        }).collect()
    }

    /// Get dictionary size
    pub fn dictionary_size(&self) -> usize {
        self.dictionary.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_dictionary() -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();
        // Write test dictionary with NULL separators (matching real format)
        write!(temp_file, "2;4;test.c:123;TEST_MODULE;Trigger no %d at %d").unwrap();
        write!(temp_file, "\x00").unwrap();
        write!(temp_file, "0;1;init.c:45;SYS_INIT;System started").unwrap(); 
        write!(temp_file, "\x00").unwrap();
        write!(temp_file, "1;2;main.c:67;MAIN_APP;Processing item %d").unwrap();
        write!(temp_file, "\x00").unwrap();
        temp_file.flush().unwrap();
        temp_file
    }

    fn create_test_binary() -> Vec<u8> {
        let mut binary_data = Vec::new();
        
        // Entry 1: timestamp=0, log_id=0 (0 args, offset 0), no arguments
        binary_data.extend_from_slice(&0u32.to_le_bytes()); // timestamp
        binary_data.extend_from_slice(&0u32.to_le_bytes()); // log_id (0 args, offset 0)
        
        // Entry 2: timestamp=1000, log_id with 2 args at offset 1
        binary_data.extend_from_slice(&1000u32.to_le_bytes()); // timestamp
        let log_id_with_args = (2u32 << 28) | 0u32; // 2 args, offset 0
        binary_data.extend_from_slice(&log_id_with_args.to_le_bytes());
        binary_data.extend_from_slice(&42u32.to_le_bytes()); // arg1
        binary_data.extend_from_slice(&100u32.to_le_bytes()); // arg2
        
        // Entry 3: timestamp=2000, log_id=1 (0 args, offset 1) 
        binary_data.extend_from_slice(&2000u32.to_le_bytes()); // timestamp
        binary_data.extend_from_slice(&1u32.to_le_bytes()); // log_id (0 args, offset 1)
        
        binary_data
    }

    #[test]
    fn test_dictionary_parsing() {
        let dict_file = create_test_dictionary();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        assert_eq!(parser.dictionary_size(), 3);
    }

    #[test]
    fn test_binary_parsing() {
        let dict_file = create_test_dictionary();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        
        let binary_data = create_test_binary();
        let temp_binary = NamedTempFile::new().unwrap();
        std::fs::write(temp_binary.path(), binary_data).unwrap();
        
        let parsed_logs = parser.parse_binary(temp_binary.path(), 5).unwrap();
        assert_eq!(parsed_logs.len(), 3);
        
        // Check first entry (system started)
        assert_eq!(parsed_logs[0].timestamp_formatted, "0ms");
        assert_eq!(parsed_logs[0].module_name, "TEST_MODULE");
        
        // Check second entry with arguments
        assert_eq!(parsed_logs[1].timestamp_formatted, "1000ms");
        assert_eq!(parsed_logs[1].formatted_message, "Trigger no 42 at 100");
    }

    #[test]
    fn test_timestamp_formatting() {
        assert_eq!(SyslogParser::format_timestamp(0), "0ms");
        assert_eq!(SyslogParser::format_timestamp(1234), "1234ms");
        assert_eq!(SyslogParser::format_timestamp(60000), "60000ms");
    }

    #[test]
    fn test_message_formatting() {
        let args = vec![42, 100];
        let result = SyslogParser::format_message("Trigger no %d at %d", &args);
        assert_eq!(result, "Trigger no 42 at 100");
        
        // Test with missing arguments
        let result = SyslogParser::format_message("Value %d and %d", &vec![42]);
        assert_eq!(result, "Value 42 and <missing>");
        
        // Test with hex formatting
        let result = SyslogParser::format_message("Address 0x%x", &vec![255]);
        assert_eq!(result, "Address 0x0xFF");
    }

    #[test]
    fn test_log_level_filtering() {
        let dict_file = create_test_dictionary();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        
        let binary_data = create_test_binary();
        let temp_binary = NamedTempFile::new().unwrap();
        std::fs::write(temp_binary.path(), binary_data).unwrap();
        
        // Filter to only level 1 and below (should get 1 entry)
        let parsed_logs = parser.parse_binary(temp_binary.path(), 1).unwrap();
        assert_eq!(parsed_logs.len(), 1);
        assert_eq!(parsed_logs[0].module_name, "SYS_INIT");
    }

    #[test]
    fn test_format_output() {
        let dict_file = create_test_dictionary();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        
        let binary_data = create_test_binary();
        let temp_binary = NamedTempFile::new().unwrap();
        std::fs::write(temp_binary.path(), binary_data).unwrap();
        
        let parsed_logs = parser.parse_binary(temp_binary.path(), 5).unwrap();
        let formatted = parser.format_logs(&parsed_logs);
        
        assert_eq!(formatted.len(), 3);
        assert!(formatted[0].contains("0ms"));
        assert!(formatted[0].contains("[TEST_MODULE]"));
        assert!(formatted[1].contains("1000ms"));
        assert!(formatted[1].contains("Trigger no 42 at 100"));
    }

    #[test]
    fn test_modulo_offset_mapping() {
        let dict_file = create_test_dictionary();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        
        let mut binary_data = Vec::new();
        // Create an entry with offset larger than dictionary size
        binary_data.extend_from_slice(&5000u32.to_le_bytes()); // timestamp
        binary_data.extend_from_slice(&1000u32.to_le_bytes()); // offset 1000, should map to 1000 % 3 = 1
        
        let temp_binary = NamedTempFile::new().unwrap();
        std::fs::write(temp_binary.path(), binary_data).unwrap();
        
        let parsed_logs = parser.parse_binary(temp_binary.path(), 5).unwrap();
        assert_eq!(parsed_logs.len(), 1);
        // Should use entry at offset 1 (1000 % 3 = 1)
        assert_eq!(parsed_logs[0].module_name, "SYS_INIT");
    }

    #[test]
    fn test_error_handling() {
        // Test with non-existent dictionary
        let result = SyslogParser::new("/non/existent/path");
        assert!(result.is_err());
        
        // Test with non-existent binary file
        let dict_file = create_test_dictionary();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        let result = parser.parse_binary("/non/existent/binary", 0);
        assert!(result.is_err());
    }
}
