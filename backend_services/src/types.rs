use std::path::PathBuf;

#[derive(serde::Deserialize)]
pub struct DecoderQuery {
    pub version: String,
    pub log_level: String,
    #[serde(default)]
    pub include_log_level: bool,
    #[serde(default)]
    pub use_custom_decoder: bool,
}

pub struct UploadedFiles {
    pub binary_file: PathBuf,
    pub custom_decoder_file: Option<PathBuf>,
}

#[derive(serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}
