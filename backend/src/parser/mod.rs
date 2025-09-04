pub mod syslog_parser;
pub mod dict_parser;
pub mod log_decoder;

pub use log_decoder::LogDecoder;
pub use syslog_parser::{ParsedData, parse_binary_data};
