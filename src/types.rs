#[derive(Clone, PartialEq)]
pub struct LogSession {
    pub id: usize,
    pub content: String,
    pub timestamp: Option<String>, // Human-readable timestamp
}
