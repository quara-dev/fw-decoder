use std::{fs, path::PathBuf};

#[derive(Clone)]
pub struct Config {
    pub downloads_path: String,
    pub temp_dir: String,
    pub bind_address: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            downloads_path: std::env::var("DOWNLOADS_PATH")
                .unwrap_or_else(|_| "/app/downloads".to_string()),
            temp_dir: std::env::var("TEMP_DIR")
                .unwrap_or_else(|_| "/tmp".to_string()),
            bind_address: std::env::var("BIND_ADDRESS")
                .unwrap_or_else(|_| "127.0.0.1:3000".to_string()),
        }
    }

    pub fn downloads_dir(&self) -> PathBuf {
        PathBuf::from(&self.downloads_path)
    }

    pub fn temp_dir(&self) -> PathBuf {
        PathBuf::from(&self.temp_dir)
    }
}

pub fn cleanup_temp_files(temp_dir: &PathBuf) -> Result<(), std::io::Error> {
    if let Ok(entries) = fs::read_dir(temp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".log") || name.ends_with(".bin") {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }
    }
    Ok(())
}
