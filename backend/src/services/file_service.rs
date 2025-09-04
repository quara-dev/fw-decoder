use std::{
    fs::File,
    io::{Write, Read},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use axum::extract::Multipart;
use crate::{config::Config, services::decoder_service::ServiceError, parser::{parse_binary_data, LogDecoder}};

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
        
        tracing::info!("Starting integrated log decoder with dictionary: {} and log level {}", dict_filename, log_level);
        
        // Read the binary file
        let mut file = File::open(input_file)
            .map_err(|e| ServiceError::IoError(e))?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| ServiceError::IoError(e))?;
        
        // Parse log level
        let log_level: i32 = log_level.parse()
            .map_err(|_| ServiceError::InvalidInput("Invalid log level".to_string()))?;
            
        // Parse binary data
        let parsed_data = parse_binary_data(&contents)
            .map_err(|e| ServiceError::InvalidInput(format!("Failed to parse binary data: {}", e)))?;
        
        // Create log decoder with dictionary
        let decoder = LogDecoder::new(dict_path.to_str().unwrap())
            .map_err(|e| ServiceError::InvalidInput(format!("Failed to load dictionary: {}", e)))?;
        
        // Decode logs
        let decoded_logs = decoder.decode_logs(parsed_data, log_level);
        
        // Join all decoded logs with newlines
        let result = decoded_logs.join("\n");
        
        tracing::info!("Log decoding completed successfully, {} entries processed", decoded_logs.len());
        
        Ok(result)
    }
}
