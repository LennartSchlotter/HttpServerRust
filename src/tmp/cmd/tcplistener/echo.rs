use std::io::{Error, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::{io::Read};
use std::sync::mpsc::{Receiver, channel};

/**
 * Echo Server
 * It might be interesting to implement this using Tokio async instead of threads.
 */
fn main() -> Result<(), Error> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Server listening on port 8080");
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                println!("Connection established");
                let mut streamclone = stream.try_clone()?;
                let receiver = get_lines(stream)?;
                for mut received in receiver {
                    println!("read: {}", received);
                    received.push('\n');
                    streamclone.write(received.as_bytes())?;
                    streamclone.flush().unwrap();
                }
                println!("Connection closed");
                return Ok(())
            },
            Err(e) => println!("Couldn't get client: {e:?}"),
        }
    }
}

fn get_lines(mut stream: TcpStream) -> Result<Receiver<String>, Error> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        let mut buf = [0u8; 8];
        let mut resultpart = String::new();
        loop {
        let data = stream.read(&mut buf).expect("Should've been able to read from stream");

        if data == 0 {
            break;
        }

        let text = String::from_utf8_lossy(&buf[..data]); 
        let mut split = text.split('\n').peekable();

        while let Some(line) = split.next() {
            resultpart.push_str(line);
            if split.peek().is_some() {
                tx.send(std::mem::take(&mut resultpart)).expect("Should've been able to send result");
                resultpart.clear();
            }
        }
        }
        if !resultpart.is_empty() {
            tx.send(resultpart).expect("Should've been able to send result");
        }
    });
    return Ok(rx);
}