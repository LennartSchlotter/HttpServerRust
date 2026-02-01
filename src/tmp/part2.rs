use std::io::{Error, Read};

/**
 * Read File, 8 Bytes a time. Output full lines.
 * The issue here is that we want to return full lines (like a HTTP Header for example) as opposed to 8 Bytes at a time. Decouple reading from presenting
 * Concepts: Buffering Strategies, Parsing vs I/O, State Machines
 * Problems: This is a very flawed implementation, looping through characters is much more expensive than 2b
 */
fn main() -> Result<(), Error> {
    let mut file = std::fs::File::open("assets/messages.txt")?;
    let mut buf = [0u8; 8];

    // initialize empty string to store not printed data across loops. Initialize outside the loop so it doesn't get reset.
    let mut resultpart = String::new();
    loop {
        let data = file.read(&mut buf)?;

        if data == 0 {
            break;
        }

        let text = str::from_utf8(&buf[..data]).expect("Should've been able to convert utf-8");

        // Iterate over all chars in the 8Bytes read.
        let chars = text.chars();
        for c in chars {
            // If no \n we keep pushing chars onto the result
            if c != '\n' {
                resultpart.push(c);
            } else {
                // as soon as we hit a linebreak, we print and clear the resultpart so we can start collecting again.
                println!("read: {}", resultpart);
                resultpart.clear();
            }
        }
    }
    println!("read: {}", resultpart);
    return Ok(())
}
