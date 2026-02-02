use std::io::{Error, Read};

/**
 * Read File, 8 Bytes at a time.
 * As HTTP messages are transmitted as streams of bytes over TCP connections, we need to learn to to read in byte chunks
 * Concepts: Buffer Management, I/O, Buffer Size vs Data Read distinction
 * Improvement todo: expect() => Proper handling, fixed size buffer length.
 */
fn main() -> Result<(), Error> {

    // Leverage open() to open file. Returns File type
    let mut file = std::fs::File::open("assets/messages.txt")?;

    // Create a buffer, 8 long. Type: u8 (1 Byte large) => 8 of these = 8 bytes
    let mut buf = [0u8; 8];
    loop {

        // Read File. Writes what was read into the buffer. Returns the size of what was read
        let data = file.read(&mut buf)?;

        // If nothing was read, we are done.
        if data == 0 {
            return Ok(());
        }

        // If something was read, read the buffer from the start all the way to the index. "data" holds the amount read, so if 3 were read we take 0-2 from the buffer
        let text = String::from_utf8_lossy(&buf[..data]); //use utf8_lossy to handle the case of characters being split in 2 bytes

        println!("read: {}", text);
    }
}
