use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use decoder::dict_log_parser::{read_syslog_dict_file, CsvRecord};
use decoder::syslog_parser::parse_binary_file;
use regex::Regex;
use std::path::{PathBuf};
use rayon::prelude::*;

#[derive(Parser, Default, Debug)]
#[clap(version = option_env!("VERGEN_GIT_DESCRIBE") , about = "A tool to parse and analyze syslog binary files.")]
struct CliArgs {
    /// The path to binary syslog from PU
    syslog_bin_file: PathBuf,

    #[clap(short, long)]
    dict_log_file: Option<PathBuf>,
    #[clap(short, long)]
    log_level: Option<i32>,


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

fn main() -> Result<()> {
    let args = CliArgs::parse();

    // Comment out the default dictionary to avoid using it
    // const DICT_FILE_CONTENTS: &str = include_str!("Quara_fw*.log");

    let syslog_bin_path = &args.syslog_bin_file.as_path().display().to_string();

    let mut data = parse_binary_file(syslog_bin_path)
        .with_context(|| format!("Error reading binary file: {}", syslog_bin_path))?;

    // Initialize records as empty vector since we don't want to use default dictionary
    let mut records: Vec<CsvRecord> = Vec::new();
       
    match args.dict_log_file {
        Some(p) => {
            let dict_file = p.as_path().display().to_string();
            println!("Using dictionary file {}", dict_file);
            records = read_syslog_dict_file(&dict_file)
                .with_context(|| format!("Error reading log dict file {}", dict_file))?;
        },
        None => {
            println!("No dictionary file provided. Use -d <dict_file> to specify a dictionary file.");
            return Ok(()); // Exit if no dictionary file is provided
        },
    }

    let mut req_log_lvl: i32 = 6;
    if let Some(l) = args.log_level {
        req_log_lvl = l;
    }

    // Process data in parallel while preserving order
    let processed_data: Vec<_> = data
        .par_iter_mut()
        .map(|value| {
            value.arg_offset = value.arg_offset.saturating_sub(1);
            let mem_offset = value.arg_offset as usize;
            if let Some(record) = records.iter().find(|r| r.mem_offset == mem_offset) {
                // Convert Vec<String> to Vec<&str> once and reuse
                let args: Vec<&str> = value.args.iter().map(|s| s.as_str()).collect();

                // Convert log_level to an integer
                let log_level: i32 = record.log_level.parse().unwrap_or(0);

                if log_level <= req_log_lvl {

                    let formatted_message = find_and_replace_printf_format_specifiers(&record.log_str, &args);
                    let colored_message = match log_level {
                        // Match a Fatal error
                        1 => formatted_message.bold().clear(),
                        // Match an error
                        2 => formatted_message.red(),
                        // Match a warning
                        3 => formatted_message.purple(),
                        // Match an info
                        4 => formatted_message.white(),
                        // Match a debug msg
                        5 => formatted_message.yellow(),
                        // Match a trace msg
                        6 => formatted_message.blue(),
                        // Default case
                        _ => formatted_message.normal(),
                    };

                    Some((value.timestamp, record.log_module.clone(), colored_message))
                
            } else {
                None
            }
        } else {
            None
        }
        })
        .collect();

    // Print the processed data
    for entry in processed_data {
        if let Some((timestamp, log_module, message)) = entry {
            println!("{}ms\t\t[{}]\t{}", timestamp, log_module, message);
        } else {
            continue;
        }
    }

    Ok(())
    }
