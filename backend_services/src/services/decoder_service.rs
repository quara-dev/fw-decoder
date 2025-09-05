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
    let downloads_dir = config.downloads_dir();
    
    let entries = fs::read_dir(&downloads_dir)
        .map_err(|_| ServiceError::NotFound("Downloads directory not found".to_string()))?;
    
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Only include .log files (dictionary files)
                if name.ends_with(".log") {
                    // Remove the .log extension for the dropdown display
                    let version_name = name.strip_suffix(".log").unwrap_or(name);
                    result.push(version_name.to_string());
                }
            }
        }
    }
    
    // Sort the results for consistent ordering
    result.sort();
    
    Ok(result)
}
