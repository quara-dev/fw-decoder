use super::{syslog_parser::ParsedData, dict_parser::{CsvRecord, read_syslog_dict_file}};
use anyhow::Result;
use regex::Regex;
use rayon::prelude::*;

pub struct LogDecoder {
    records: Vec<CsvRecord>,
}

impl LogDecoder {
    pub fn new(dict_file_path: &str) -> Result<Self> {
        let records = read_syslog_dict_file(dict_file_path)?;
        Ok(Self { records })
    }

    pub fn decode_logs(&self, mut data: Vec<ParsedData>, log_level: i32) -> Vec<String> {
        // Process data in parallel while preserving order
        let processed_data: Vec<_> = data
            .par_iter_mut()
            .map(|value| {
                value.arg_offset = value.arg_offset.saturating_sub(1);
                let mem_offset = value.arg_offset as usize;
                
                if let Some(record) = self.records.iter().find(|r| r.mem_offset == mem_offset) {
                    // Convert Vec<String> to Vec<&str> once and reuse
                    let args: Vec<&str> = value.args.iter().map(|s| s.as_str()).collect();

                    // Convert log_level to an integer
                    let record_log_level: i32 = record.log_level.parse().unwrap_or(0);

                    if record_log_level <= log_level {
                        let formatted_message = find_and_replace_printf_format_specifiers(&record.log_str, &args);
                        Some((value.timestamp, record.log_module.clone(), formatted_message))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Convert to formatted strings
        processed_data
            .into_iter()
            .filter_map(|entry| {
                entry.map(|(timestamp, log_module, message)| {
                    format!("{}ms\t\t[{}]\t{}", timestamp, log_module, message)
                })
            })
            .collect()
    }
}

fn find_and_replace_printf_format_specifiers(input: &str, replacements: &[&str]) -> String {
    // Define the regex pattern for printf format specifiers
    let re = Regex::new(r"%[-+ #0]*\d*(\.\d+)?[diuoxXfFeEgGaAcspn]").unwrap();

    // Iterator over the replacements
    let mut replacement_iter = replacements.iter();
    let replacer = |_: &regex::Captures| replacement_iter.next().unwrap_or(&"").to_string();

    // Replace each format specifier with the corresponding replacement
    re.replace_all(input, replacer).to_string().replace("\"", "")
}
