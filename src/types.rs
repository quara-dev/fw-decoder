use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct LogSession {
    pub id: usize,
    pub content: String,
    pub timestamp: Option<String>, // Human-readable timestamp
}
