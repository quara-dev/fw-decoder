use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use axum::extract::Multipart;
use crate::{config::Config, services::decoder_service::ServiceError};

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

    pub fn run_decoder(&self, input_file: &PathBuf, decoder_version: &str, log_level: &str) -> Result<String, ServiceError> {
        let decoder_path = self.config.decoders_dir().join(decoder_version);
        let log_path = input_file.with_extension("log");
        
        // Check if decoder exists
        if !decoder_path.exists() {
            return Err(ServiceError::NotFound(format!("Decoder '{}' not found", decoder_version)));
        }
        
        let output = Command::new(&decoder_path)
            .arg(input_file.to_str().unwrap())
            .arg("-l")
            .arg(log_level)
            .stdout(File::create(&log_path)?)
            .output()
            .map_err(|e| ServiceError::IoError(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ServiceError::InvalidInput(format!("Decoder error: {}", stderr)));
        }
        
        self.read_output_file(&log_path)
    }

    fn read_output_file(&self, log_path: &PathBuf) -> Result<String, ServiceError> {
        let bytes = std::fs::read(log_path)?;
        
        // Try to convert to UTF-8, fallback to hex
        match String::from_utf8(bytes.clone()) {
            Ok(text) => Ok(text),
            Err(_) => {
                let hex = bytes
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                Ok(format!("[binary output]\n{}", hex))
            }
        }
    }
}
