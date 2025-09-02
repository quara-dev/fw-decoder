#[derive(serde::Deserialize)]
pub struct DecoderQuery {
    pub version: String,
    pub log_level: String,
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
