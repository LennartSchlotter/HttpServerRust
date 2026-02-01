use std::io::{self, Error};
use std::thread;
use std::{fs::File, io::Read};
use std::sync::mpsc::{self, SendError};

use thiserror::Error;

/**
 * Read File, 8 Bytes a time. Output full lines. SPLIT ON LINEBREAKS VER.
 * As HTTP servers handle multiple connections, we need to adress that. 
 * Concepts: Threads / Synchronization, Channels, Ownership Transfer, Resource Management
 * Problems: OS Threads are expensive, Buffer is stack allocated (consider Vector)
 */
fn main() -> Result<(), Error> {
    let file = std::fs::File::open("assets/messages.txt")?;

    // Channels in Rust are for communication between threads. They allow a unidirectional flow of information between Sender and Receiver (tx and rx)
    let (tx, rx) = mpsc::channel();
    
    // Spawn a thread and move ownership of file and tx into the closure. That means, once the thread finishes, they are dropped (file closes, sender dropped)
    // Each OS thread requires its own stack (1-8 MB) and involves kernel-level context switching, which can cause overhead 
    // Using one thread per HTTP request can work but becomes inefficient at scale
    // Threads are beneficial due to their safety (Garbage collection, data race prevention)
    // moving contents (file, tx) to the thread, meaning as that closes, the files / sender get cleaned up automatically
    thread::spawn(move || {
        get_lines(file, tx).expect("Should've been able to retrieve lines from channel");
    });

    // The Receiver will contain the strings. For every string received, print it.
    for received in rx {
        println!("read: {}", received);
    }
    Ok(())
}

/// Function to encapsulate the retrieving logic.
fn get_lines(mut file: File, tx: mpsc::Sender<String>) -> Result<(), Error> {
    let mut buf = [0u8; 8];
    let mut resultpart = String::new();
    loop {
        let data = file.read(&mut buf)?;

        if data == 0 {
            break;
        }

        let text = String::from_utf8_lossy(&buf[..data]);
        let mut split = text.split('\n').peekable();

        while let Some(line) = split.next() {
            resultpart.push_str(line);
            if split.peek().is_some() {
                // rather than printing, send the resultpart (that would've been printed) to the channel. The logic is identical other than that.
                //Using mem::take here for cheap transfer + clean
                tx.send(std::mem::take(&mut resultpart)).expect("Should've been able to send message into channel");
                resultpart.clear();
            }
        }
    }
    if !resultpart.is_empty() {
        tx.send(resultpart).expect("Should've been able to send leftovers to channel");
    }
    Ok(())
}