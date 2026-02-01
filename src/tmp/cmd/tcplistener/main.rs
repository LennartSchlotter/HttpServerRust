use std::{net::TcpListener};

use crate::internal::request::request::{HttpError, request_from_reader};

/**
 * TCP Listener
 * It might be interesting to implement this using Tokio async instead of threads.
 */
fn main() -> Result<(), HttpError>{
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Server listening on port 8080");
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                println!("Connection established");
                let request = request_from_reader(&mut stream)?;
                print!("- Method: {} \n", request.request_line.method);
                print!("- Target: {} \n", request.request_line.request_target);
                print!("- Version: {} \n", request.request_line.http_version);
                println!("Headers:");
                for (key, val) in request.headers.iter() {
                    println!("{key}: {val}");
                }
                println!("\r\n");
                println!("Body:"); //Should this line print if there is no body?
                match String::from_utf8(request.body) {
                    Ok(string) => println!("{}", string),
                    Err(error) => eprintln!("Body has invalid UTF-8: {}", error),
                };
                println!("Connection closed");
                return Ok(())
            },
            Err(e) => println!("Couldn't get client: {e:?}"),
        }
    }
}
