use std::io::{Write, Seek};
use tempfile::NamedTempFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Let's just calculate the byte positions manually
    let entry1 = "2;4;test.c:123;TEST_MODULE;Trigger no %d at %d";
    let entry2 = "0;1;init.c:45;SYS_INIT;System started"; 
    let entry3 = "1;2;main.c:67;MAIN_APP;Processing item %d";
    
    println!("Entry 0: '{}' - length: {} bytes", entry1, entry1.len());
    println!("Entry 0 starts at byte: 0");
    
    let offset1 = entry1.len() + 1; // +1 for NULL separator
    println!("Entry 1: '{}' - length: {} bytes", entry2, entry2.len());
    println!("Entry 1 starts at byte: {}", offset1);
    
    let offset2 = offset1 + entry2.len() + 1; // +1 for NULL separator
    println!("Entry 2: '{}' - length: {} bytes", entry3, entry3.len());
    println!("Entry 2 starts at byte: {}", offset2);
    
    // Now let's create the actual file and verify
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", entry1)?;
    write!(temp_file, "\x00")?;
    write!(temp_file, "{}", entry2)?;
    write!(temp_file, "\x00")?;
    write!(temp_file, "{}", entry3)?;
    write!(temp_file, "\x00")?;
    temp_file.flush()?;
    
    // Read back and verify offsets
    let contents = std::fs::read(temp_file.path())?;
    println!("\nVerification - file has {} bytes total", contents.len());
    
    // Find NULL positions
    for (i, &b) in contents.iter().enumerate() {
        if b == 0 {
            println!("NULL found at byte {}", i);
        }
    }
    
    Ok(())
}
