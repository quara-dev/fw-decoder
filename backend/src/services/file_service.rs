use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH, Duration},
};
use axum::extract::Multipart;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
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

    pub async fn run_decoder(&self, input_file: &PathBuf, decoder_version: &str, log_level: &str) -> Result<String, ServiceError> {
        let decoder_path = self.config.decoders_dir().join(decoder_version);
        let log_path = input_file.with_extension("log");
        
        // Check if decoder exists
        if !decoder_path.exists() {
            return Err(ServiceError::NotFound(format!("Decoder '{}' not found", decoder_version)));
        }
        
        tracing::info!("Starting decoder execution: {} with log level {}", decoder_version, log_level);
        
        // Create output file for decoder stdout
        let output_file = File::create(&log_path)
            .map_err(|e| ServiceError::IoError(e))?;
        
        // Run decoder with timeout (30 minutes max)
        let mut command = TokioCommand::new(&decoder_path);
        command
            .arg(input_file.to_str().unwrap())
            .arg("-l")
            .arg(log_level)
            .stdout(output_file)
            .stderr(std::process::Stdio::piped());

        tracing::info!("Decoder process spawned, waiting for completion...");

        // Set timeout to 30 minutes for large files
        let timeout_duration = Duration::from_secs(30 * 60);
        
        let result = timeout(timeout_duration, command.output()).await;
        
        match result {
            Ok(Ok(output)) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::error!("Decoder failed with error: {}", stderr);
                    return Err(ServiceError::InvalidInput(format!("Decoder error: {}", stderr)));
                }
                
                tracing::info!("Decoder completed successfully, reading output...");
                self.read_output_file(&log_path)
            }
            Ok(Err(e)) => {
                tracing::error!("Decoder process error: {}", e);
                Err(ServiceError::IoError(e))
            }
            Err(_) => {
                tracing::error!("Decoder process timed out after 30 minutes");
                Err(ServiceError::InvalidInput("Decoder process timed out. The file might be too large or corrupted.".to_string()))
            }
        }
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
