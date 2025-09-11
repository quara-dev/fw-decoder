use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH, Duration},
};
use axum::extract::Multipart;
use syslog_decoder::SyslogParser;
use tokio::time::timeout;
use crate::{
    config::Config, 
    services::decoder_service::ServiceError, 
    parser::session_parser::parse_log_sessions,
    types::UploadedFiles,
};

// Resource management constants
const PROCESSING_TIMEOUT: Duration = Duration::from_secs(45 * 60); // 45 minutes for very large files
const MAX_UPLOAD_SIZE: usize = 500 * 1024 * 1024; // 500MB upload limit

pub struct FileProcessor {
    config: Config,
}

impl FileProcessor {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn process_upload(&self, mut multipart: Multipart) -> Result<UploadedFiles, ServiceError> {
        let temp_dir = self.config.temp_dir();
        
        // Clean up previous temp files
        crate::config::cleanup_temp_files(&temp_dir)?;

        let mut binary_file: Option<PathBuf> = None;
        let mut custom_decoder_file: Option<PathBuf> = None;

        while let Some(mut field) = multipart
            .next_field()
            .await
            .map_err(|e| ServiceError::InvalidInput(format!("Invalid multipart data: {}", e)))?
        {
            if let Some(field_name) = field.name() {
                let field_name = field_name.to_string();
                
                if let Some(filename) = field.file_name() {
                    let filename = filename.to_string();
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    let temp_filename = format!("{}_{}", now, filename);
                    let filepath = temp_dir.join(&temp_filename);
                    
                    // Use streaming approach for large files
                    let mut buffer = Vec::new();
                    let mut total_size = 0;
                    
                    // Read in smaller chunks to avoid memory issues
                    while let Some(chunk) = field.chunk().await.map_err(|e| {
                        ServiceError::InvalidInput(format!("Failed to read file chunk: {}", e))
                    })? {
                        total_size += chunk.len();
                        
                        // Check upload size limit early
                        if total_size > MAX_UPLOAD_SIZE {
                            return Err(ServiceError::InvalidInput(
                                format!("File too large: {} bytes (max: {} bytes)", 
                                       total_size, MAX_UPLOAD_SIZE)
                            ));
                        }
                        
                        buffer.extend_from_slice(&chunk);
                    }
                    
                    // Write the entire buffer to file at once
                    std::fs::write(&filepath, &buffer)
                        .map_err(|e| ServiceError::IoError(e))?;
                    
                    println!("Uploaded {} file: {} ({:.2} MB)", field_name, filename, total_size as f64 / (1024.0 * 1024.0));
                    
                    // Assign to appropriate field based on field name
                    match field_name.as_str() {
                        "file" => binary_file = Some(filepath),
                        "custom_decoder" => custom_decoder_file = Some(filepath),
                        _ => {
                            // Default to binary file if field name is unknown
                            if binary_file.is_none() {
                                binary_file = Some(filepath);
                            }
                        }
                    }
                }
            }
        }
        
        match binary_file {
            Some(binary_path) => Ok(UploadedFiles {
                binary_file: binary_path,
                custom_decoder_file,
            }),
            None => Err(ServiceError::InvalidInput("No binary file found in upload".to_string())),
        }
    }

    pub async fn run_decoder(&self, input_file: &PathBuf, firmware_version: &str, log_level: &str, _include_log_level: bool, custom_decoder_file: Option<&PathBuf>) -> Result<String, ServiceError> {
        // Determine which dictionary file to use
        let dict_path = if let Some(custom_file) = custom_decoder_file {
            // Use the custom decoder file
            custom_file.clone()
        } else {
            // Use the firmware version to find the corresponding dictionary file in downloads
            let dict_filename = format!("{}.log", firmware_version);
            let dict_path = self.config.downloads_dir().join(&dict_filename);
            
            // Check if dictionary file exists
            if !dict_path.exists() {
                return Err(ServiceError::NotFound(
                    format!("Dictionary file not found: {}. Please refresh the files or provide a custom decoder file.", dict_filename)
                ));
            }
            dict_path
        };
        
        // Get the dictionary filename for logging
        let dict_filename = dict_path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("custom_decoder");
        
        println!("Starting syslog parser library with dictionary: {} and log level {} (always including log levels in response)", dict_filename, log_level);
        
        // Parse log level
        let log_level_num: u8 = log_level.parse()
            .map_err(|_| ServiceError::InvalidInput("Invalid log level".to_string()))?;
        
        // Run decoder with timeout protection
        let result = timeout(PROCESSING_TIMEOUT, async {
            // Create syslog parser with dictionary
            let parser = SyslogParser::new(&dict_path)
                .map_err(|e| ServiceError::InvalidInput(format!("Failed to load dictionary: {}", e)))?;
            
            // Parse binary file (this now handles large files with streaming)
            let parsed_logs = parser.parse_binary(input_file, log_level_num)
                .map_err(|e| ServiceError::InvalidInput(format!("Failed to parse binary file: {}", e)))?;
            
            // Always format logs with log levels - frontend will control display
            let formatted_logs = parser.format_logs_with_options(&parsed_logs, true);
            
            // Join all formatted logs with newlines for session parsing
            let decoded_text = formatted_logs.join("\n");
            
            // Parse into sessions
            let sessions = parse_log_sessions(&decoded_text);
            
            // Return sessions as JSON
            let sessions_json = serde_json::to_string(&sessions)
                .map_err(|e| ServiceError::InvalidInput(format!("Failed to serialize sessions: {}", e)))?;
            
            println!("Syslog parsing completed successfully, {} logs processed, {} sessions created", 
                     formatted_logs.len(), sessions.len());
            
            Ok::<String, ServiceError>(sessions_json)
        }).await;
        
        match result {
            Ok(decoder_result) => decoder_result,
            Err(_) => Err(ServiceError::InvalidInput(
                format!("Processing timed out after {} minutes. File may be too large or corrupted.", 
                       PROCESSING_TIMEOUT.as_secs() / 60)
            ))
        }
    }
}
