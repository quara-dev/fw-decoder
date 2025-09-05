use super::{syslog_parser::ParsedData, dict_parser::{CsvRecord, read_syslog_dict_file}};
use anyhow::{Context, Result};
use rayon::prelude::*;
use regex::Regex;
use std::sync::Arc;

/// Enhanced log decoder with optimizations and better error handling
pub struct LogDecoder {
    /// Dictionary records for message template lookup
    records: Vec<CsvRecord>,
    /// Compiled regex for format specifiers (shared across threads)
    format_regex: Arc<Regex>,
    /// Configuration options
    config: DecoderConfig,
}

/// Configuration options for the log decoder
#[derive(Debug, Clone)]
pub struct DecoderConfig {
    /// Whether to include timestamps in output
    pub include_timestamps: bool,
    /// Whether to include module names in output
    pub include_modules: bool,
    /// Maximum number of logs to process (0 = no limit)
    pub max_logs: usize,
    /// Whether to include statistics in output
    pub include_stats: bool,
    /// Custom timestamp format function
    pub timestamp_formatter: Option<fn(u32) -> String>,
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            include_timestamps: true,
            include_modules: true,
            max_logs: 0,
            include_stats: false,
            timestamp_formatter: Some(|ts| format!("{}ms", ts)),
        }
    }
}

/// Decoded log entry with metadata
#[derive(Debug, Clone)]
pub struct DecodedLogEntry {
    /// Original timestamp from binary log
    pub timestamp: u32,
    /// Formatted log message
    pub message: String,
    /// Log level
    pub log_level: String,
    /// Module name
    pub module: String,
    /// Original memory offset
    pub mem_offset: usize,
}

impl DecodedLogEntry {
    /// Format the log entry for display
    pub fn format(&self, config: &DecoderConfig) -> String {
        let mut parts = Vec::new();

        // Add timestamp if configured
        if config.include_timestamps {
            let timestamp_str = if let Some(formatter) = config.timestamp_formatter {
                formatter(self.timestamp)
            } else {
                format!("{}", self.timestamp)
            };
            parts.push(timestamp_str);
        }

        // Add module if configured
        if config.include_modules && !self.module.is_empty() {
            parts.push(format!("[{}]", self.module));
        }

        // Add the message
        parts.push(self.message.clone());

        parts.join("\t\t")
    }
}

impl LogDecoder {
    /// Create a new LogDecoder with the specified dictionary
    pub fn new(dict_file_path: &str) -> Result<Self> {
        let records = read_syslog_dict_file(dict_file_path)
            .with_context(|| format!("Failed to load dictionary from {}", dict_file_path))?;

        // Compile regex once for better performance
        let format_regex = Arc::new(
            Regex::new(r"%[-+ #0]*\d*(\.\d+)?[diuoxXfFeEgGaAcspn]")
                .context("Failed to compile format specifier regex")?
        );

        Ok(Self {
            records,
            format_regex,
            config: DecoderConfig::default(),
        })
    }

    /// Create a LogDecoder with custom configuration
    pub fn with_config(dict_file_path: &str, config: DecoderConfig) -> Result<Self> {
        let mut decoder = Self::new(dict_file_path)?;
        decoder.config = config;
        Ok(decoder)
    }

    /// Decode parsed log data into human-readable format with optimizations
    pub fn decode_logs(&self, mut data: Vec<ParsedData>, log_level: i32) -> Vec<String> {
        // Apply max_logs limit if specified
        if self.config.max_logs > 0 && data.len() > self.config.max_logs {
            data.truncate(self.config.max_logs);
        }

        // Process data in parallel while preserving order
        let processed_data: Vec<_> = data
            .par_iter_mut()
            .filter_map(|value| {
                // Adjust offset (original logic preserved)
                value.arg_offset = value.arg_offset.saturating_sub(1);
                let mem_offset = value.arg_offset as usize;
                
                // Find matching record efficiently
                if let Some(record) = self.find_record_by_offset(mem_offset) {
                    // Parse and check log level
                    let record_log_level: i32 = record.log_level.parse().unwrap_or(0);

                    if record_log_level <= log_level {
                        // Convert args efficiently
                        let args: Vec<&str> = value.args.iter().map(|s| s.as_str()).collect();

                        // Format message with optimized function
                        let formatted_message = self.format_message_optimized(&record.log_str, &args);
                        
                        Some(DecodedLogEntry {
                            timestamp: value.timestamp,
                            message: formatted_message,
                            log_level: record.log_level.clone(),
                            module: record.log_module.clone(),
                            mem_offset,
                        })
                    } else {
                        None
                    }
                } else {
                    // Handle unknown offsets gracefully
                    Some(DecodedLogEntry {
                        timestamp: value.timestamp,
                        message: format!("Unknown log format [offset: 0x{:08x}]", mem_offset),
                        log_level: "UNKNOWN".to_string(),
                        module: "UNKNOWN".to_string(),
                        mem_offset,
                    })
                }
            })
            .collect();

        // Convert to formatted strings
        let mut result: Vec<String> = processed_data
            .iter()
            .map(|entry| entry.format(&self.config))
            .collect();

        // Add statistics if requested
        if self.config.include_stats {
            result.push(format!(
                "\n=== Decoding Statistics ===\nTotal entries processed: {}\nDictionary entries: {}\nFiltered by log level: {}",
                processed_data.len(),
                self.records.len(),
                log_level
            ));
        }

        result
    }

    /// Find record by offset with optimized search
    #[inline]
    fn find_record_by_offset(&self, offset: usize) -> Option<&CsvRecord> {
        // For small datasets, linear search is often faster than HashMap lookup
        // due to better cache locality
        self.records.iter().find(|record| record.mem_offset == offset)
    }

    /// Optimized message formatting with better error handling
    fn format_message_optimized(&self, format_str: &str, args: &[&str]) -> String {
        let mut arg_iter = args.iter();
        
        let result = self.format_regex.replace_all(format_str, |_caps: &regex::Captures| {
            arg_iter.next().unwrap_or(&"<missing>").to_string()
        });

        // Remove quotes and clean up the result
        result.to_string().replace("\"", "")
    }

    /// Get decoder statistics
    pub fn get_stats(&self) -> DecoderStats {
        DecoderStats {
            dictionary_entries: self.records.len(),
            config: self.config.clone(),
        }
    }

    /// Update decoder configuration
    pub fn set_config(&mut self, config: DecoderConfig) {
        self.config = config;
    }
}

/// Statistics about the decoder
#[derive(Debug)]
pub struct DecoderStats {
    pub dictionary_entries: usize,
    pub config: DecoderConfig,
}

/// Legacy function for backward compatibility (optimized version)
pub fn find_and_replace_printf_format_specifiers(input: &str, replacements: &[&str]) -> String {
    // Use the optimized regex pattern
    let re = Regex::new(r"%[-+ #0]*\d*(\.\d+)?[diuoxXfFeEgGaAcspn]").unwrap();
    
    let mut replacement_iter = replacements.iter();
    let result = re.replace_all(input, |_: &regex::Captures| {
        replacement_iter.next().unwrap_or(&"").to_string()
    });

    result.to_string().replace("\"", "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_message_optimized() {
        // This test would require a full decoder setup, so we'll test the standalone function
        let result = find_and_replace_printf_format_specifiers("Hello %s, number %d", &["World", "42"]);
        assert_eq!(result, "Hello World, number 42");
    }

    #[test]
    fn test_decoder_config_default() {
        let config = DecoderConfig::default();
        assert!(config.include_timestamps);
        assert!(config.include_modules);
        assert_eq!(config.max_logs, 0);
        assert!(!config.include_stats);
    }
}
