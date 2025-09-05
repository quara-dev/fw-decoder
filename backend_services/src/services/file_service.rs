use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use axum::extract::Multipart;
use syslog_decoder::SyslogParser;
use crate::{
    config::Config, 
    services::decoder_service::ServiceError, 
    parser::session_parser::parse_log_sessions
};

pub struct FileProcessor {
    config: Config,
}

impl FileProcessor {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn process_upload(&self, mut multipart: Multipart) -> Result<PathBuf, ServiceError> {
        let temp_dir = self.config.temp_dir();
        
        // Clean up previous temp files
        crate::config::cleanup_temp_files(&temp_dir)?;

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|_| ServiceError::InvalidInput("Invalid multipart data".to_string()))?
        {
            if let Some(filename) = field.file_name() {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let temp_filename = format!("{}_{}", now, filename);
                let filepath = temp_dir.join(&temp_filename);
                
                let data = field
                    .bytes()
                    .await
                    .map_err(|_| ServiceError::InvalidInput("Failed to read file data".to_string()))?;
                
                let mut file = File::create(&filepath)?;
                file.write_all(&data)?;
                
                return Ok(filepath);
            }
        }
        
        Err(ServiceError::InvalidInput("No file found in upload".to_string()))
    }

    pub async fn run_decoder(&self, input_file: &PathBuf, firmware_version: &str, log_level: &str) -> Result<String, ServiceError> {
        // Use the firmware version to find the corresponding dictionary file in downloads
        let dict_filename = format!("{}.log", firmware_version);
        let dict_path = self.config.downloads_dir().join(&dict_filename);
        
        // Check if dictionary file exists
        if !dict_path.exists() {
            return Err(ServiceError::NotFound(format!("Dictionary file '{}' not found in downloads", dict_filename)));
        }
        
        println!("Starting syslog parser library with dictionary: {} and log level {}", dict_filename, log_level);
        
        // Parse log level
        let log_level_num: u8 = log_level.parse()
            .map_err(|_| ServiceError::InvalidInput("Invalid log level".to_string()))?;
        
        // Create syslog parser with dictionary
        let parser = SyslogParser::new(&dict_path)
            .map_err(|e| ServiceError::InvalidInput(format!("Failed to load dictionary: {}", e)))?;
        
        // Parse binary file
        let parsed_logs = parser.parse_binary(input_file, log_level_num)
            .map_err(|e| ServiceError::InvalidInput(format!("Failed to parse binary file: {}", e)))?;
        
        // Format logs into strings
        let formatted_logs = parser.format_logs(&parsed_logs);
        
        // Join all formatted logs with newlines for session parsing
        let decoded_text = formatted_logs.join("\n");
        
        // Parse into sessions
        let sessions = parse_log_sessions(&decoded_text);
        
        // Return sessions as JSON
        let sessions_json = serde_json::to_string(&sessions)
            .map_err(|e| ServiceError::InvalidInput(format!("Failed to serialize sessions: {}", e)))?;
        
        println!("Syslog parsing completed successfully, {} logs processed, {} sessions created", 
                 formatted_logs.len(), sessions.len());
        
        Ok(sessions_json)
    }
}
