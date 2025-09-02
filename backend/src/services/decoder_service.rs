use std::fs;
use crate::config::Config;

#[derive(Debug)]
pub enum ServiceError {
    IoError(std::io::Error),
    NotFound(String),
    InvalidInput(String),
}

impl From<std::io::Error> for ServiceError {
    fn from(err: std::io::Error) -> Self {
        ServiceError::IoError(err)
    }
}

pub fn get_available_decoders(config: &Config) -> Result<Vec<String>, ServiceError> {
    let mut result = Vec::new();
    let decoders_dir = config.decoders_dir();
    
    let entries = fs::read_dir(&decoders_dir)
        .map_err(|_| ServiceError::NotFound("Decoders directory not found".to_string()))?;
    
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Only include files that start with 'decoder_'
                if name.starts_with("decoder_") {
                    result.push(name.to_string());
                }
            }
        }
    }
    
    Ok(result)
}
