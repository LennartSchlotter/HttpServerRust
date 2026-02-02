use std::io::{Error, Read};

/**
 * Read File, 8 Bytes a time. Output full lines. SPLIT ON LINEBREAKS VER.
 * The issue here is that we want to return full lines (like a HTTP Header for example) as opposed to 8 Bytes at a time. Decouple reading from presenting
 * Concepts: Buffering Strategies, Parsing vs I/O, State Machines
 */
fn main() -> Result<(), Error> {
    let mut file = std::fs::File::open("assets/messages.txt")?;
    let mut buf = [0u8; 8];
    let mut resultpart = String::new();
    loop {
        let data = file.read(&mut buf)?;

        if data == 0 {
            break;
        }

        let text = str::from_utf8(&buf[..data]).expect("Should've been able to convert utf-8");

        // Split the text on \n, creating 1-N strings to iterate over. If there is no \n, the full string is returned, else we always get multiple parts with the
        // \n removed. So if The line ENDS on \n, we still get 2 strings, one full line and then an empty string.
        // Use peekable to be able to peek inside the iterator as we loop through the strings created
        let mut split = text.split('\n').peekable();

        // Loop through the strings, always writing the next iteration into "line"
        while let Some(line) = split.next() {

            // Push line onto the resultpart. As we split on every linebreak, this is safe.
            resultpart.push_str(line);

            // The last element is always one that does not need printing. Either it's an empty string or a substring AFTER a linebreak that is not followed by a linebreak.
            if split.peek().is_some() {
                
                // as such, if another element follows, we are safe to print. If no other element follows, skip this and just keep pushing line onto resultpart
                println!("read: {}", resultpart);

                // After printing, we need to clear resultpart
                resultpart.clear();
            }
        }
    }

    // If the file ends on a \n, it would print an extra empty read
    if !resultpart.is_empty(){
        println!("read: {}", resultpart);
    }
    Ok(())
}