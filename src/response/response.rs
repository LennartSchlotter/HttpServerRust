use std::{fmt::{self}, io::{self}};

use crate::{headers::headers::Headers, request::request::HttpError};

/// Representation of a HTTP response with status code, headers and body
pub struct Response {
    pub status: StatusCode,
    pub headers: Headers,
    pub body: Vec<u8>,
}

/// Enum containing the valid status codes used in this application.
// FIXME Will likely be expanded in the future.
#[derive(Clone, Copy, Debug)]
pub enum StatusCode {
    Ok = 200,
    Created = 201,
    BadRequest = 400,
    NotFound = 404,
    InternalServerError = 500,
}

/// Implements Display for the Status Code to enable formatting the Codes as integer values.
impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u16)
    }
}

impl StatusCode {
    
    /// Creates the string representation of the passed status code.
    pub fn reason_phrase(&self) -> &str {
        match self {
            StatusCode::Ok => "OK",
            StatusCode::Created => "Created",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::NotFound => "Not Found",
            StatusCode::InternalServerError => "Internal Server Error",
        }
    }
}

/// Write the status line to the passed writer.
/// 
/// Hardcodes HTTP/1.1 due to the limit of the Server to that version.
pub fn write_status_line<W: io::Write>(mut writer: W, status_code: StatusCode ) -> std::io::Result<()>{
    write!(writer, "HTTP/1.1 {} {}\r\n", status_code as u16, status_code.reason_phrase())?;
    Ok(())
}

/// Writes the headers to the passed writer.
/// 
/// Given a hashmap of headers, iterates through them and prints the keys and values in HTTP valid format.
/// Also prints the final linebreak separating headers from the HTTP body.
pub fn write_headers<W: io::Write>(mut writer: W, headers: &mut Headers) -> std::io::Result<()> {
    for (key, value) in headers.iter() {
        writer.write_all(format!("{}: {}\r\n", key, value).as_bytes())?;
    }
    writer.write_all(b"\r\n")?;
    Ok(())
}

/// Writes the body in chunks
/// 
/// # Output
/// [Length in Hex]\r\n
/// 
/// [Data]\r\n
pub fn write_chunked_body<W: io::Write>(mut writer: W, data: &[u8]) -> Result<(), HttpError> {
    let hex = format!("{:X}\r\n", data.len());
    writer.write_all(hex.as_bytes())?;

    writer.write_all(data)?;
    writer.write_all("\r\n".as_bytes())?;
    Ok(())
}

/// Writes the final part of the body if passed with chunked transfer encoding.
/// 
/// This is standardized.
/// 
/// # Example
/// ...
/// 
/// 0\r\n
/// 
/// \r\n
pub fn write_final_body_chunk<W: io::Write>(mut writer: W, trailers: Option<Headers>) -> Result<(), HttpError> {
    writer.write_all("0\r\n".as_bytes())?;
    match trailers {
        Some(trailers) => {
            write_trailers(&mut writer, trailers).unwrap(); //error handling
        }
        None => writer.write_all("\r\n".as_bytes())?,
    }
    Ok(())
}

/// Identical function to write_headers, kept for readability
pub fn write_trailers<W: io::Write>(mut writer: W, headers: Headers) -> Result<(), HttpError> {
    for (key, value) in headers.iter() {
        writer.write_all(format!("{}: {}\r\n", key.to_lowercase(), value.to_lowercase()).as_bytes())?;
    }
    writer.write_all(b"\r\n")?;
    Ok(())
}

/// Helper function to remove boilerplate for creating html responses with associated headers.
pub fn html_response(status_code: StatusCode, html: &str) -> Response {
    let mut headers = Headers::new();
    headers.insert("content-type", "text/html");
    headers.insert("content-length", html.len().to_string());
    return Response { status: status_code, headers: headers, body: html.as_bytes().to_vec()};
}