use anyhow::Result;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug)]
pub struct CsvRecord {
    pub args_num: String,
    pub log_level: String,
    pub log_src_line: String,
    pub log_module: String,
    pub log_str: String,
    pub mem_offset: usize,
}

impl CsvRecord {
    pub fn find_by_mem_offset(records: &[CsvRecord], offset: usize) -> Option<&CsvRecord> {
        records.iter().find(|&record| record.mem_offset == offset)
    }
}

pub fn read_syslog_dict_file(file_path: &str) -> Result<Vec<CsvRecord>> {
    // Open the binary file
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);

    // Vector to store all records
    let mut records = Vec::new();
    let mut cumulative_length = 0;

    // read the whole file
    let mut line = String::new();
    while reader.read_line(&mut line)? != 0 {
        // let line = line?;
        let mut line = std::mem::take(&mut line);
        line.pop();
        let fields: Vec<&str> = line.split(';').collect();
        if fields.len() >= 5 {
            let line_length = line.len();
            //println!("{} : {}", cumulative_length, fields[4]);
            records.push(CsvRecord {
                args_num: fields[0].to_string(),
                log_level: fields[1].to_string(),
                log_src_line: fields[2].to_string(),
                log_module: fields[3].to_string(),
                log_str: fields[4].to_string(),
                mem_offset: cumulative_length,
            });
            cumulative_length += line_length + 1; // +1 for the newline character
        }
    }

    Ok(records)
}


pub fn read_syslog_dict(file_path: &str) -> Result<Vec<CsvRecord>> {
    // Open the binary file
    // let file = File::open(file_path).context("Failed to open log dict file")?;
    // let reader = BufReader::new(file);
    // const DICT_FILE_CONTENTS: &str = include_str!(file);

    // Vector to store all records
    let mut records = Vec::new();
    let mut cumulative_length = 0;

    // Iterate through the lines and parse each field
    for line in file_path.lines() {
        // let line = line?;
        let fields: Vec<&str> = line.split(';').collect();
        if fields.len() >= 5 {
            let line_length = line.len();
            records.push(CsvRecord {
                args_num: fields[0].to_string(),
                log_level: fields[1].to_string(),
                log_src_line: fields[2].to_string(),
                log_module: fields[3].to_string(),
                log_str: fields[4].to_string(),
                mem_offset: cumulative_length,
            });
            cumulative_length += line_length + 1; // +1 for the newline character
        }
    }

    Ok(records)
}
