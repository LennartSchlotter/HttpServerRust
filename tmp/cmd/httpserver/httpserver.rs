use std::{io::Error, sync::Arc};

use crate::internal::{request::request::{HttpError, Request}, response::response::{Response, StatusCode, html_response}, server::{handler::Handler, server::serve}};

struct MyHandler;

/**
 * Minimal implementation, accepting any endpoint
 */
impl Handler for MyHandler {
    fn call<W: std::io::Write>(&self, request: &Request, stream: &mut W) -> Result<Option<Response>, HttpError> {
        match request.request_line.request_target.as_str() {
            _ => {
                let body = "<html><body><h1>All good!</h1></body></html>";
                let response = html_response(StatusCode::Ok, body);
                return Ok(Some(response));
            }
        }
    }
}

/**
 * Example usage implementation.
 */
fn main() -> Result<(), Error> {
    const PORT: u16 = 8080;
    let handler = MyHandler;
    let handler_arc = Arc::new(handler);
    serve(PORT, handler_arc)?;
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf)?;
    Ok(())
}