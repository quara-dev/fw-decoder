use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, BufReader};
use std::path::Path;
use anyhow::{Result, Context};
use regex::Regex;

// Resource optimization constants for large file handling
const CHUNK_SIZE: usize = 16 * 1024 * 1024;  // 16MB chunks for binary reading
const MAX_ENTRIES_PER_BATCH: usize = 10000;  // Process entries in batches 
const PROGRESS_REPORT_INTERVAL: usize = 100000; // Report progress every 100k entries
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2GB file size limit

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
    pub log_level: u8,
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

/// Syslog parser library with optimized parsing
pub struct SyslogParser {
    dictionary: HashMap<u32, LogEntry>,
    // Store raw dictionary content for byte-offset lookups
    raw_dictionary: Vec<u8>,
}

impl SyslogParser {
    /// Create a new parser with dictionary file
    pub fn new<P: AsRef<Path>>(dictionary_path: P) -> Result<Self> {
        let (dictionary, raw_dictionary) = Self::load_dictionary(dictionary_path)?;
        
        Ok(Self { 
            dictionary,
            raw_dictionary,
        })
    }

    /// Load dictionary from .log file (optimized with byte offset support)
    fn load_dictionary<P: AsRef<Path>>(path: P) -> Result<(HashMap<u32, LogEntry>, Vec<u8>)> {
        let contents = fs::read(&path)
            .with_context(|| format!("Failed to read dictionary file: {}", path.as_ref().display()))?;
        
        let mut dictionary = HashMap::new();

        // Split by NULL character (0x00) and track byte positions
        let mut start_pos = 0;
        for end_pos in contents.iter().enumerate().filter_map(|(i, &b)| if b == 0x00 { Some(i) } else { None }) {
            if start_pos < end_pos {
                let entry_bytes = &contents[start_pos..end_pos];
                let line = String::from_utf8_lossy(entry_bytes);
                let trimmed = line.trim();
                
                if !trimmed.is_empty() {
                    match Self::parse_dictionary_line(trimmed) {
                        Ok(entry) => {
                            dictionary.insert(start_pos as u32, entry);
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to parse dictionary line at byte {}: {} ({})", 
                                     start_pos, trimmed, e);
                        }
                    }
                }
            }
            
            start_pos = end_pos + 1; // Skip the NULL character
        }

        // Handle the last entry if file doesn't end with NULL
        if start_pos < contents.len() {
            let entry_bytes = &contents[start_pos..];
            let line = String::from_utf8_lossy(entry_bytes);
            let trimmed = line.trim();
            
            if !trimmed.is_empty() {
                match Self::parse_dictionary_line(trimmed) {
                    Ok(entry) => {
                        dictionary.insert(start_pos as u32, entry);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse dictionary line at byte {}: {} ({})", 
                                 start_pos, trimmed, e);
                    }
                }
            }
        }

        println!("Loaded {} dictionary entries from {}", 
                 dictionary.len(), path.as_ref().display());
        Ok((dictionary, contents))
    }

    /// Get dictionary entry by byte offset from raw dictionary content
    fn get_entry_by_byte_offset(&self, byte_offset: u32) -> Option<LogEntry> {
        let offset = byte_offset as usize;
        if offset >= self.raw_dictionary.len() {
            return None;
        }

        // Find the end of this entry (next NULL character or end of file)
        let mut end_pos = offset;
        while end_pos < self.raw_dictionary.len() && self.raw_dictionary[end_pos] != 0x00 {
            end_pos += 1;
        }

        if end_pos == offset {
            return None; // Empty entry
        }

        let entry_bytes = &self.raw_dictionary[offset..end_pos];
        let line = String::from_utf8_lossy(entry_bytes);
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return None;
        }

        match Self::parse_dictionary_line(trimmed) {
            Ok(entry) => Some(entry),
            Err(e) => {
                eprintln!("Warning: Failed to parse dictionary entry at byte offset {}: {} ({})", 
                         byte_offset, trimmed, e);
                None
            }
        }
    }

    /// Parse a single dictionary line (optimized)
    /// Format: num_args;log_level;source_file:line_number;module_name;log_message
    fn parse_dictionary_line(line: &str) -> Result<LogEntry> {
        let mut parts = line.splitn(5, ';'); // More efficient - stops after 5 parts
        
        // Skip num_args (parts[0])
        parts.next().context("Missing num_args field")?;

        let log_level = parts.next()
            .context("Missing log_level field")?
            .trim()
            .parse::<u8>()
            .context("Failed to parse log level")?;

        // Skip source file and line number (parts[2])
        parts.next().context("Missing source_file field")?;
        
        let module_name = parts.next()
            .context("Missing module_name field")?
            .trim()
            .to_string();
        
        let log_message = parts.next()
            .context("Missing log_message field")?
            .trim()
            .to_string();

        Ok(LogEntry {
            log_level,
            module_name,
            log_message,
        })
    }

    /// Parse binary log file and return formatted logs (optimized for large files)
    pub fn parse_binary<P: AsRef<Path>>(&self, binary_path: P, min_log_level: u8) -> Result<Vec<ParsedLog>> {
        // Check file size first
        let metadata = std::fs::metadata(&binary_path)
            .with_context(|| format!("Failed to get file metadata: {}", binary_path.as_ref().display()))?;
        
        if metadata.len() > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!("File too large: {} bytes (max: {} bytes)", 
                                     metadata.len(), MAX_FILE_SIZE));
        }

        println!("Parsing binary file: {} ({:.2} MB)", 
                 binary_path.as_ref().display(), 
                 metadata.len() as f64 / (1024.0 * 1024.0));

        // Use streaming reader for large files, regular reader for small files
        if metadata.len() > CHUNK_SIZE as u64 {
            self.parse_binary_streaming(binary_path, min_log_level)
        } else {
            self.parse_binary_legacy(binary_path, min_log_level)
        }
    }

    /// Legacy method for small files (loads entire file into memory)
    fn parse_binary_legacy<P: AsRef<Path>>(&self, binary_path: P, min_log_level: u8) -> Result<Vec<ParsedLog>> {
        let binary_entries = self.read_binary_file_legacy(binary_path)?;
        
        let mut parsed_logs = Vec::with_capacity(binary_entries.len().min(MAX_ENTRIES_PER_BATCH));

        for entry in binary_entries {
            if let Some(parsed_log) = self.process_binary_entry(&entry, min_log_level) {
                parsed_logs.push(parsed_log);
            }
        }

        println!("Parsed {} logs from binary file (min level: {})", 
                 parsed_logs.len(), min_log_level);
        Ok(parsed_logs)
    }

    /// Streaming method for large files (processes in chunks)
    fn parse_binary_streaming<P: AsRef<Path>>(&self, binary_path: P, min_log_level: u8) -> Result<Vec<ParsedLog>> {
        let file = File::open(&binary_path)
            .with_context(|| format!("Failed to open binary file: {}", binary_path.as_ref().display()))?;
        
        let mut reader = BufReader::new(file);
        let mut parsed_logs = Vec::new();
        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut remainder = Vec::new();
        let mut total_entries = 0;
        let mut batch_count = 0;

        loop {
            // Read chunk from file
            let bytes_read = reader.read(&mut buffer)
                .with_context(|| "Failed to read from binary file")?;
            
            if bytes_read == 0 {
                break; // End of file
            }

            // Combine remainder from previous chunk with new data
            let mut chunk_data = remainder;
            chunk_data.extend_from_slice(&buffer[..bytes_read]);

            // Process entries from this chunk
            let (entries, remaining_bytes) = self.parse_chunk(&chunk_data)?;
            
            // Process entries in batches to manage memory
            for batch in entries.chunks(MAX_ENTRIES_PER_BATCH) {
                for entry in batch {
                    if let Some(parsed_log) = self.process_binary_entry(entry, min_log_level) {
                        parsed_logs.push(parsed_log);
                    }
                    total_entries += 1;

                    // Report progress periodically
                    if total_entries % PROGRESS_REPORT_INTERVAL == 0 {
                        println!("Processed {} entries...", total_entries);
                    }
                }
                
                batch_count += 1;
                // Hint that batch processing is complete for memory management
                if batch_count % 10 == 0 {
                    // Allow garbage collector to reclaim memory from processed batches
                    println!("Processed {} batches, {} entries total", batch_count, total_entries);
                }
            }

            // Save incomplete data for next iteration
            remainder = remaining_bytes;

            // If we're at end of file but have remaining bytes, it's incomplete data
            if bytes_read < CHUNK_SIZE && !remainder.is_empty() {
                println!("Warning: {} incomplete bytes at end of file", remainder.len());
                break;
            }
        }

        println!("Streaming parse completed: {} logs from {} total entries (min level: {})", 
                 parsed_logs.len(), total_entries, min_log_level);
        Ok(parsed_logs)
    }

    /// Parse binary entries from a chunk of data, returning entries and any remaining bytes
    fn parse_chunk(&self, data: &[u8]) -> Result<(Vec<BinaryLogEntry>, Vec<u8>)> {
        let mut entries = Vec::new();
        let mut offset = 0;

        while offset + 8 <= data.len() {
            // Read timestamp (32-bit)
            let timestamp_ms = u32::from_le_bytes([
                data[offset],
                data[offset + 1], 
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;

            // Read log_id (32-bit)
            let log_id_raw = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2], 
                data[offset + 3],
            ]);
            offset += 4;

            // Extract number of arguments and log offset
            let num_args = ((log_id_raw >> 28) & 0xF) as u8;
            let log_offset = log_id_raw & 0x0FFFFFFF;

            // Check if we have enough data for all arguments
            let args_size = num_args as usize * 4;
            if offset + args_size > data.len() {
                // Not enough data for arguments - return remaining data
                let remaining = data[offset - 8..].to_vec(); // Include current entry header
                return Ok((entries, remaining));
            }

            // Read arguments
            let mut arguments = Vec::with_capacity(num_args as usize);
            for _ in 0..num_args {
                let arg = u32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                arguments.push(arg);
                offset += 4;
            }

            entries.push(BinaryLogEntry {
                timestamp_ms,
                log_id: log_offset,
                arguments,
            });
        }

        // Return any remaining bytes that couldn't form a complete entry
        let remaining = if offset < data.len() {
            data[offset..].to_vec()
        } else {
            Vec::new()
        };

        Ok((entries, remaining))
    }

    /// Read and parse binary file structure (legacy method for small files)
    fn read_binary_file_legacy<P: AsRef<Path>>(&self, path: P) -> Result<Vec<BinaryLogEntry>> {
        let contents = fs::read(&path)
            .with_context(|| format!("Failed to read binary file: {}", path.as_ref().display()))?;

        // Pre-allocate vector with estimated capacity (each entry is min 8 bytes)
        let mut entries = Vec::with_capacity(contents.len() / 8);
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

    /// Process a single binary entry and create formatted log (updated for byte offset)
    fn process_binary_entry(&self, entry: &BinaryLogEntry, min_log_level: u8) -> Option<ParsedLog> {
        // Use byte offset directly instead of modulo mapping
        let log_entry = self.get_entry_by_byte_offset(entry.log_id)?;

        // Filter by log level
        if log_entry.log_level > min_log_level {
            return None;
        }

        // Format timestamp
        let timestamp_formatted = Self::format_timestamp(entry.timestamp_ms);

        // Format message with arguments
        let formatted_message = self.format_message(&log_entry.log_message, &entry.arguments);

        Some(ParsedLog {
            timestamp_formatted,
            log_level: log_entry.log_level,
            module_name: log_entry.module_name.clone(),
            formatted_message,
        })
    }

    /// Format timestamp from milliseconds to readable format matching expected output
    fn format_timestamp(timestamp_ms: u32) -> String {
        format!("{}ms", timestamp_ms)
    }

    /// Format log message by replacing placeholders with arguments (optimized)
    fn format_message(&self, template: &str, arguments: &[u32]) -> String {
        let mut result = template.to_string();
        let mut arg_index = 0;

        // First handle consecutive hex pattern "0x%x%x%x..." (at least 2 %x) -> "0x32304644"
        let consecutive_hex_pattern = Regex::new(r"0x%x(?:%x)+").unwrap(); // Matches 0x%x followed by at least one more %x
        let mut replacements = Vec::new();
        
        for mat in consecutive_hex_pattern.find_iter(&result) {
            let full_match = mat.as_str();
            let hex_count = full_match.matches("%x").count();
            
            if arg_index + hex_count <= arguments.len() {
                let mut hex_string = String::from("0x");
                for _ in 0..hex_count {
                    hex_string.push_str(&format!("{:02X}", arguments[arg_index] & 0xFF));
                    arg_index += 1;
                }
                replacements.push((mat.range(), hex_string));
            } else {
                replacements.push((mat.range(), "<missing>".to_string()));
            }
        }
        
        // Apply replacements in reverse order to maintain indices
        for (range, replacement) in replacements.into_iter().rev() {
            result.replace_range(range, &replacement);
        }

        // Now handle remaining individual placeholders
        let combined_pattern = Regex::new(r"%(?:l{0,2}([udx])|([s]))").unwrap();
        
        result = combined_pattern.replace_all(&result, |caps: &regex::Captures| {
            let placeholder = if let Some(long_match) = caps.get(1) {
                long_match.as_str()
            } else if let Some(string_match) = caps.get(2) {
                string_match.as_str()
            } else {
                "unknown"
            };
            
            if arg_index < arguments.len() {
                let value = match placeholder {
                    "d" => arguments[arg_index].to_string(),
                    "u" => arguments[arg_index].to_string(), 
                    "x" => format!("0x{:X}", arguments[arg_index]),
                    "s" => "<string>".to_string(),
                    _ => "<unknown>".to_string(),
                };
                arg_index += 1;
                value
            } else {
                "<missing>".to_string()
            }
        }).to_string();

        result
    }

    /// Convert log level number to descriptive string
    fn log_level_to_string(level: u8) -> &'static str {
        match level {
            0 => "Critical",
            1 => "FatalError",
            2 => "Error",
            3 => "Warning",
            4 => "Info",
            5 => "Debug",
            6 => "Verbose",
            _ => "Unknown",
        }
    }

    /// Get formatted output as strings for compatibility (optimized)
    pub fn format_logs(&self, logs: &[ParsedLog]) -> Vec<String> {
        self.format_logs_with_options(logs, false)
    }

    /// Get formatted output as strings with option to include log level
    pub fn format_logs_with_options(&self, logs: &[ParsedLog], include_log_level: bool) -> Vec<String> {
        logs.iter().map(|log| {
            if include_log_level {
                format!("{:12}\t[{}]\t[{}]\t{}", 
                       log.timestamp_formatted,
                       Self::log_level_to_string(log.log_level),
                       log.module_name,
                       log.formatted_message)
            } else {
                format!("{:12}\t[{}]\t{}", 
                       log.timestamp_formatted,
                       log.module_name,
                       log.formatted_message)
            }
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
        
        // Entry 1: timestamp=0, log_id=0 (0 args, byte offset 0), no arguments
        binary_data.extend_from_slice(&0u32.to_le_bytes()); // timestamp
        binary_data.extend_from_slice(&0u32.to_le_bytes()); // log_id (0 args, byte offset 0)
        
        // Entry 2: timestamp=1000, log_id with 2 args at byte offset 0 (first entry)
        binary_data.extend_from_slice(&1000u32.to_le_bytes()); // timestamp
        let log_id_with_args = (2u32 << 28) | 0u32; // 2 args, byte offset 0
        binary_data.extend_from_slice(&log_id_with_args.to_le_bytes());
        binary_data.extend_from_slice(&42u32.to_le_bytes()); // arg1
        binary_data.extend_from_slice(&100u32.to_le_bytes()); // arg2
        
        // Entry 3: timestamp=2000, log_id=47 (0 args, byte offset 47 for SYS_INIT entry) 
        binary_data.extend_from_slice(&2000u32.to_le_bytes()); // timestamp
        binary_data.extend_from_slice(&47u32.to_le_bytes()); // log_id (0 args, byte offset 47)
        
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
        let dict_file = create_test_dictionary();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        
        let args = vec![42, 100];
        let result = parser.format_message("Trigger no %d at %d", &args);
        assert_eq!(result, "Trigger no 42 at 100");
        
        // Test with missing arguments
        let result = parser.format_message("Value %d and %d", &vec![42]);
        assert_eq!(result, "Value 42 and <missing>");
        
        // Test with hex formatting
        let result = parser.format_message("Address 0x%x", &vec![255]);
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
    fn test_byte_offset_mapping() {
        let dict_file = create_test_dictionary();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        
        let mut binary_data = Vec::new();
        // Create an entry that uses byte offset to reference the second entry
        binary_data.extend_from_slice(&5000u32.to_le_bytes()); // timestamp
        
        // Second entry "0;1;init.c:45;SYS_INIT;System started" starts at byte 47
        let second_entry_offset = 47u32;
        binary_data.extend_from_slice(&second_entry_offset.to_le_bytes()); // byte offset 47
        
        let temp_binary = NamedTempFile::new().unwrap();
        std::fs::write(temp_binary.path(), binary_data).unwrap();
        
        let parsed_logs = parser.parse_binary(temp_binary.path(), 5).unwrap();
        assert_eq!(parsed_logs.len(), 1);
        // Should use entry at byte offset 47 (SYS_INIT entry)
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

    #[test]
    fn test_log_level_in_output() {
        let dict_file = create_test_dictionary();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        
        let binary_data = create_test_binary();
        let temp_binary = NamedTempFile::new().unwrap();
        std::fs::write(temp_binary.path(), binary_data).unwrap();
        
        let parsed_logs = parser.parse_binary(temp_binary.path(), 5).unwrap();
        
        // Test formatting without log level (default behavior)
        let formatted_without_level = parser.format_logs(&parsed_logs);
        assert!(formatted_without_level[0].contains("[TEST_MODULE]"));
        assert!(!formatted_without_level[0].contains("[Warning]")); // Should not contain log level
        
        // Test formatting with log level
        let formatted_with_level = parser.format_logs_with_options(&parsed_logs, true);
        assert!(formatted_with_level[0].contains("[Info]\t[TEST_MODULE]")); // Should contain log level "Info" (level 4)
        assert!(formatted_with_level[2].contains("[FatalError]\t[SYS_INIT]")); // Should contain log level "FatalError" (level 1)
        
        // Verify structure: timestamp\t[log_level]\t[module]\tmessage
        let parts: Vec<&str> = formatted_with_level[0].split('\t').collect();
        assert_eq!(parts.len(), 4);
        assert!(parts[1].starts_with('[') && parts[1].ends_with(']')); // log level in brackets
        assert!(parts[2].starts_with('[') && parts[2].ends_with(']')); // module in brackets
    }

    #[test]
    fn test_log_level_strings() {
        // Test all log level string mappings
        assert_eq!(SyslogParser::log_level_to_string(0), "Critical");
        assert_eq!(SyslogParser::log_level_to_string(1), "FatalError");
        assert_eq!(SyslogParser::log_level_to_string(2), "Error");
        assert_eq!(SyslogParser::log_level_to_string(3), "Warning");
        assert_eq!(SyslogParser::log_level_to_string(4), "Info");
        assert_eq!(SyslogParser::log_level_to_string(5), "Debug");
        assert_eq!(SyslogParser::log_level_to_string(6), "Verbose");
        assert_eq!(SyslogParser::log_level_to_string(255), "Unknown"); // Test unknown level
    }

    #[test]
    fn test_unsigned_placeholder() {
        let dict_file = create_test_dictionary_with_unsigned();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        
        // Test %u (unsigned) formatting
        let result = parser.format_message("Date time set rcvd: %u", &vec![1234567890]);
        assert_eq!(result, "Date time set rcvd: 1234567890");
        
        // Test %lu (long unsigned) formatting
        let result = parser.format_message("Free space in workspace volume : (%lu kb / %lu kb)", &vec![1024, 2048]);
        assert_eq!(result, "Free space in workspace volume : (1024 kb / 2048 kb)");
        
        // Test mixed placeholders including %lu
        let result = parser.format_message("Event %d at time %u with status 0x%x and size %lu", &vec![42, 1234567890, 255, 1024]);
        assert_eq!(result, "Event 42 at time 1234567890 with status 0x0xFF and size 1024");
        
        // Test %lu with missing argument
        let result = parser.format_message("Size: %lu", &vec![]);
        assert_eq!(result, "Size: <missing>");
    }

    fn create_test_dictionary_with_unsigned() -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();
        // Write test dictionary with %u placeholder
        write!(temp_file, "1;4;protocol.c:123;SYS_PROTOCOL_DATE_TIME;Date time set rcvd: %u").unwrap();
        write!(temp_file, "\x00").unwrap();
        temp_file.flush().unwrap();
        temp_file
    }

    #[test]
    fn test_long_format_specifiers() {
        let dict_file = create_test_dictionary_with_unsigned();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        
        // Test various long format specifiers
        let result = parser.format_message("Long unsigned: %lu", &vec![4294967295]);
        assert_eq!(result, "Long unsigned: 4294967295");
        
        let result = parser.format_message("Long decimal: %ld", &vec![123456]);
        assert_eq!(result, "Long decimal: 123456");
        
        let result = parser.format_message("Long hex: %lx", &vec![255]);
        assert_eq!(result, "Long hex: 0xFF");
        
        // Test double long format specifiers (should also work)
        let result = parser.format_message("Long long: %llu", &vec![9999]);
        assert_eq!(result, "Long long: 9999");
        
        // Test mixed format specifiers
        let result = parser.format_message("Values: %d %u %x %lu %ld", &vec![1, 2, 3, 4, 5]);
        assert_eq!(result, "Values: 1 2 0x3 4 5");
    }

    #[test]
    fn test_consecutive_hex_formatting() {
        let dict_file = create_test_dictionary();
        let parser = SyslogParser::new(dict_file.path()).unwrap();
        
        // Test consecutive %x formatting (should be combined into single hex value)
        let result = parser.format_message("Session is ....0x%x%x%x%x", &vec![0x32, 0x30, 0x46, 0x44]);
        assert_eq!(result, "Session is ....0x32304644");
        
        // Test individual %x (should have separate 0x prefix)
        let result = parser.format_message("Address %x and value %x", &vec![0x32, 0x44]);
        assert_eq!(result, "Address 0x32 and value 0x44");
        
        // Test mixed case
        let result = parser.format_message("ID: 0x%x%x, Status: %x", &vec![0xAB, 0xCD, 0xFF]);
        assert_eq!(result, "ID: 0xABCD, Status: 0xFF");
    }
}
