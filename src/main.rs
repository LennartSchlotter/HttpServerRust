//! # Rust HTTP Server
//! 
//! This binary crate provides a HTTP server implementation built on top of the library in this crate
//! 
//! It supports basic request parsing and response generation.
//! 
//! Refer to the library documentation of reusable components.
use std::{io::Read, sync::Arc};
use sha2::{Sha256, Digest};

use httpserver::{headers::headers::Headers, request::request::{HttpError, Request}, response::response::{Response, StatusCode, html_response, write_chunked_body, write_final_body_chunk, write_headers, write_status_line}, server::{handler::Handler, server::serve}};

struct MyHandler;

/**
 * Example Implementation. This is more to showcase usage rather than be a realistic depiction of the server's usage.
 * To do is still to improve options the server provides to clean this implementation up.
 */
impl Handler for MyHandler {
    fn call<W: std::io::Write>(&self, request: &Request, mut stream: &mut W) -> Result<Option<Response>, HttpError> {
        match request.request_line.request_target.as_str() { //can also match request.request_line.method to differentiate between GET, POST etc
            "/yourproblem" => {
                let body = "<html><body><h1>Bad Request</h1></body></html>"; //fs::read_to_string("example/400.html")
                let response = html_response(StatusCode::BadRequest, body);
                Ok(Some(response))
            },
            "/myproblem" => {
                let body = "<html><body><h1>Internal Server Error</h1></body></html>";
                let response = html_response(StatusCode::InternalServerError, body);
                Ok(Some(response))
            },
            path if path.starts_with("/httpbin/stream/") => {
                let suffix = path.strip_prefix("/httpbin/stream/").unwrap(); //can safely unwrap
                let url = "https://httpbin.org/stream/".to_string() + suffix;

                let mut response = reqwest::blocking::get(url).expect("Should've been able to retrieve data from the URL");
                write_status_line(&mut stream, StatusCode::BadRequest)?;
                let mut headers = Headers::new();
                headers.insert("content-type", "text/plain");
                headers.insert("transfer-encoding", "chunked");
                headers.insert("trailer", "X-Content-SHA256, X-Content-Length");
                write_headers(&mut stream, &mut headers)?;

                let mut full_body = Vec::new();
                loop {
                    let mut buffer = [0u8; 1024];
                    let data = response.read(&mut buffer)?;

                    if data == 0 {
                        break;
                    }

                    write_chunked_body(&mut stream,  &buffer[..data])?;
                    full_body.extend_from_slice(&buffer[..data]); //This is expensive just to compute the hash, could update the hash on each iteration
                }
                if headers.get("trailer").is_some() {
                    let mut trailers = Headers::new();
                    let mut hasher = Sha256::new();
                    hasher.update(&full_body);
                    let digest = hex::encode(hasher.finalize());
                    trailers.insert("X-Content-SHA256", digest);
                    trailers.insert("X-Content-Length", full_body.len().to_string());
                    write_final_body_chunk(&mut stream, Some(trailers))?;
                } else {
                    write_final_body_chunk(&mut stream, None)?;
                }
                Ok(None)
            },
            "/mp4" => {
                let file = std::fs::read("assets/video.mp4")?;
                let mut headers = Headers::new();
                headers.insert("content-type", "video/mp4");
                headers.insert("content-length", file.len().to_string());
                Ok(Some(Response { status: StatusCode::Ok, headers, body: file }))
            },
            _ => {
                let body = "<html><body><h1>All good!</h1></body></html>";
                let response = html_response(StatusCode::Ok, body);
                Ok(Some(response))
            }
        }
    }
}

fn main() -> Result<(), HttpError> {
    const PORT: u16 = 8080;
    let handler = MyHandler;
    let handler_arc = Arc::new(handler);
    serve(PORT, handler_arc)?;
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf)?;
    Ok(())
}