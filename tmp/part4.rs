use std::io::Error;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::{io::Read};
use std::sync::mpsc::{Receiver, channel};

/**
 * TCP Listener
 * It might be interesting to implement this using Tokio async instead of threads.
 */
fn main() -> Result<(), Error> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Server listening on port 8080");
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                println!("Connection established");
                let receiver = get_lines(stream)?;
                for received in receiver {
                    println!("read: {}", received);
                }
                println!("Connection closed");
                return Ok(())
            },
            Err(e) => println!("Couldn't get client: {e:?}"),
        }
    }
}

/// Function to encapsulate the retrieving logic.
fn get_lines(mut stream: TcpStream) -> Result<Receiver<String>, Error> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        let mut buf = [0u8; 16];
        let mut resultpart = String::new();
        loop {
        let data = match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };

        if data == 0 {
            break;
        }

        let text = String::from_utf8_lossy(&buf[..data]);
        let mut split = text.split("\r\n").peekable();

        while let Some(line) = split.next() {
            resultpart.push_str(line);
            if split.peek().is_some() {
                tx.send(std::mem::take(&mut resultpart)).expect("Should've been able to send line to channel");
                resultpart.clear();
            }
        }
        }
        if !resultpart.is_empty() {
            tx.send(resultpart).expect("Should've been able to send leftovers to channel");
        }
    });
    return Ok(rx);
}