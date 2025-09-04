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
                // Only include .log files
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

pub fn map_firmware_version_to_decoder(firmware_version: &str) -> Option<String> {
    // Extract version numbers from firmware filename
    // Examples: "Quara_fw_9.17.3.13" -> use decoder_9_17_3_1
    //           "Quara_fw_9.12.2.0" -> use decoder_9_12_2_0
    //           "Quara_fw_9.6.1.0" -> use decoder_9_6_1_0
    
    if let Some(version_part) = firmware_version.strip_prefix("Quara_fw_") {
        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() >= 3 {
            let major = parts[0];
            let minor = parts[1];
            let patch = parts[2];
            
            // Map to available decoders based on version compatibility
            match (major, minor, patch) {
                ("9", "17", "3") => Some("decoder_9_17_3_1".to_string()),
                ("9", "17", "2") => Some("decoder_9_17_2_1".to_string()), // or decoder_9_17_2_2
                ("9", "12", "2") => Some("decoder_9_12_2_0".to_string()),
                ("9", "6", "1") => Some("decoder_9_6_1_0".to_string()),
                _ => {
                    // Default fallback - try to find the closest decoder
                    if major == "9" && minor == "17" {
                        Some("decoder_9_17_3_1".to_string()) // Use latest 9.17.x decoder
                    } else if major == "9" && minor == "12" {
                        Some("decoder_9_12_2_0".to_string())
                    } else if major == "9" && minor == "6" {
                        Some("decoder_9_6_1_0".to_string())
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        }
    } else {
        None
    }
}
