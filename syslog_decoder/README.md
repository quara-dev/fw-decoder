# Syslog Decoder Library

An independent Rust library for parsing firmware syslog files with NULL-separated dictionary entries and binary log data.

## Features

- **Dictionary Parsing**: Handles NULL-separated (0x00) dictionary files with log templates
- **Binary Log Parsing**: Processes binary log files with timestamp and argument extraction
- **Boot Cycle Detection**: Compatible with session parsing for boot cycle boundaries
- **Argument Replacement**: Supports %d, %x, %s placeholder replacement in log messages
- **Log Level Filtering**: Filter logs by minimum log level
- **Modulo Offset Mapping**: Handles dictionary offset mismatches gracefully

## Usage

### As Library

```rust
use syslog_decoder::SyslogParser;

// Create parser with dictionary file
let parser = SyslogParser::new("firmware_dict.log")?;

// Parse binary logs with minimum log level 5
let parsed_logs = parser.parse_binary("syslog.bin", 5)?;

// Get formatted output
let formatted = parser.format_logs(&parsed_logs);
for log in formatted {
    println!("{}", log);
}
```

### As Standalone Binary

```bash
# Build the library
cargo build --release

# Run the parser
cargo run --bin syslog_parser -- dictionary.log binary.bin 5
```

## File Formats

### Dictionary Format
```
num_args;log_level;source_file:line_number;module_name;log_message<NULL>
```

Example:
```
2;4;main.c:123;SYS_MODULE;Processing item %d with value %d<NULL>
0;1;init.c:45;BOOT;System started<NULL>
```

### Binary Format
- 4 bytes: timestamp (little-endian u32)  
- 4 bytes: log_id (28-bit offset + 4-bit arg_count)
- N × 4 bytes: arguments (little-endian u32)

## Output Format
```
{timestamp}ms        [{module}]      {formatted_message}
```

Example:
```
0ms             [BOOT]          System started
1250ms          [SYS_MODULE]    Processing item 42 with value 100
```

## Testing

```bash
cargo test
```

## Dependencies

- `anyhow`: Error handling
- `tempfile`: Test utilities (dev-dependency)

## License

MIT OR Apache-2.0
