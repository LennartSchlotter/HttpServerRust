use crate::request::request::HttpError;

/// A Http Request Line representation with method, target and http version
#[derive(Debug)]
pub struct RequestLine {
    pub method: String,
    pub request_target: String,
    pub http_version: String,
}

/// Parses a passed string into a Request Line Struct
/// 
/// Returns an Optional Request Line in case the passed string did not contain the entire line
/// Returns the size of the parsed data to differentiate between unfinished parsing and completion.
/// 
/// Throws an HTTP Error if invalid
pub fn parse_request_line(request: &str) -> Result<(Option<RequestLine>, usize), HttpError> {
    const VALID_METHODS: &[&str] = &["GET", "POST", "PATCH", "PUT", "DELETE", "HEAD", "OPTIONS", "CONNECT", "TRACE"];
    const CRLF_LEN: usize = 2;

    if !request.contains("\r\n") {
        return Ok((None, 0));
    }

    let mut line = request.split("\r\n");
    let first = line.next().unwrap();
    let parts: Vec<&str> = first.split(" ").collect();

    // Also ensures below [i] checks cannot panic and end the application, else could also use explitic .next() and handle mnaually.
    // parts.next().ok_or(HttpError::MalformedRequestLine)?
    if parts.len() != 3 {
        return Err(HttpError::MalformedRequestLine)
    }

    let method = parts[0].to_string();
    let request_target = parts[1].to_string();
    let http_version = parts[2].strip_prefix("HTTP/").ok_or(HttpError::MalformedRequestLine)?.to_string();

    if !VALID_METHODS.contains(&method.as_str()) {
        return Err(HttpError::InvalidMethod(method))
    }

    let line_length = first.len() + CRLF_LEN;

    Ok((Some(RequestLine { method, request_target, http_version }), line_length))
}