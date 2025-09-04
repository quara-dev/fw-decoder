use anyhow::{Context, Result};
use std::fs::File;
use std::io::Read;
use std::convert::TryInto;

#[derive(Debug)]
pub struct ParsedData {
    pub timestamp: u32,
    pub num_args: u8,
    pub arg_offset: u32,
    pub args: Vec<String>,
}


pub fn parse_binary_file(file_path: &str) -> Result<Vec<ParsedData>> {
    // Open the binary file
    let mut file = File::open(file_path).context("Failed to open binary file")?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).context("Failed to read binary file")?;

    let mut parsed_data = Vec::with_capacity(contents.len() / 12); // Rough estimate of capacity
    let mut offset = 0;

    while offset < contents.len() {
        // Parse the timestamp
        let timestamp = u32::from_le_bytes(contents[offset..offset + 4].try_into().context("Failed to parse timestamp")?);
        offset += 4;

        // Parse the second u32
        let second_u32 = u32::from_le_bytes(contents[offset..offset + 4].try_into().context("Failed to parse second u32")?);
        offset += 4;

        let num_args = (second_u32 >> 28) as u8;
        let arg_offset = second_u32 & 0x0FFFFFFF;

        // Parse the arguments
        let mut args = Vec::with_capacity(num_args as usize);
        for _ in 0..num_args {
            if offset + 4 > contents.len() {
                return Err(anyhow::anyhow!("Not enough bytes to parse argument"));
            }
            let arg = u32::from_le_bytes(contents[offset..offset + 4].try_into().context("Failed to parse argument")?);
            offset += 4;
            args.push(arg.to_string());
        }

        parsed_data.push(ParsedData {
            timestamp,
            num_args,
            arg_offset,
            args,
        });
    }

    Ok(parsed_data)
}