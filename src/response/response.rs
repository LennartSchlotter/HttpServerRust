use std::{fmt::{self}, io::{self}};

use crate::{headers::headers::Headers, request::request::HttpError};

/// Representation of a HTTP response with status code, headers and body
#[derive(Debug)]
pub struct Response {
    /// The status code the response contains
    pub status: StatusCode,
    /// The headers the response contains
    pub headers: Headers,
    /// A byte vector representing the body
    pub body: Vec<u8>,
}

/// Enum containing the valid status codes used in this application.
#[derive(Clone, Copy, Debug)]
pub enum StatusCode {
    /// Represents a successful response
    Ok = 200,
    /// Represents a successful creation
    Created = 201,
    /// Represents an invalid request
    BadRequest = 400,
    /// Represents the request target not being found as a valid endpoint
    NotFound = 404,
    /// Represents an internal error of the server
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
    #[must_use]
    pub const fn reason_phrase(&self) -> &str {
        match self {
            Self::Ok => "OK",
            Self::Created => "Created",
            Self::BadRequest => "Bad Request",
            Self::NotFound => "Not Found",
            Self::InternalServerError => "Internal Server Error",
        }
    }
}

/// Write the status line to the passed writer.
/// 
/// Hardcodes HTTP/1.1 due to the limit of the Server to that version.
/// 
/// # Errors
/// 
/// This function will return an `HttpError::Io` if the underlying writer fails to write the entire buffer.
pub fn write_status_line<W: io::Write>(mut writer: W, status_code: StatusCode ) -> io::Result<()>{
    write!(writer, "HTTP/1.1 {} {}\r\n", status_code as u16, status_code.reason_phrase())?;
    Ok(())
}

/// Writes the headers to the passed writer.
/// 
/// Given a hashmap of headers, iterates through them and prints the keys and values in HTTP valid format.
/// Also prints the final linebreak separating headers from the HTTP body.
/// 
/// # Errors
/// 
/// This function will return an `HttpError::Io` if the underlying writer fails to write the entire buffer.
pub fn write_headers<W: io::Write>(mut writer: W, headers: &mut Headers) -> io::Result<()> {
    for (key, value) in headers.iter() {
        writer.write_all(format!("{key}: {value}\r\n").as_bytes())?;
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
/// 
/// # Errors
/// 
/// This function will return an `HttpError::Io` if any write operation to the underlying writer fails.
pub fn write_chunked_body<W: io::Write>(mut writer: W, data: &[u8]) -> Result<(), HttpError> {
    let hex = format!("{:X}\r\n", data.len());
    writer.write_all(hex.as_bytes())?;

    writer.write_all(data)?;
    writer.write_all(b"\r\n")?;
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
/// 
/// # Errors
/// 
/// This function will return an `HttpError::Io` if any write operation to the underlying writer fails.
pub fn write_final_body_chunk<W: io::Write>(mut writer: W, trailers: Option<Headers>) -> Result<(), HttpError> {
    writer.write_all(b"0\r\n")?;
    match trailers {
        Some(trailers) => {
            write_trailers(&mut writer, &trailers)?;
        }
        None => writer.write_all(b"\r\n")?,
    }
    Ok(())
}

/// Identical function to `write_headers`, kept for readability
/// 
/// # Errors
/// 
/// This function will return an `HttpError::Io` if any write operation to the underlying writer fails
pub fn write_trailers<W: io::Write>(mut writer: W, headers: &Headers) -> Result<(), HttpError> {
    for (key, value) in headers.iter() {
        writer.write_all(format!("{}: {}\r\n", key.to_lowercase(), value.to_lowercase()).as_bytes())?;
    }
    writer.write_all(b"\r\n")?;
    Ok(())
}

/// Helper function to remove boilerplate for creating html responses with associated headers.
#[must_use]
pub fn html_response(status: StatusCode, html: &str) -> Response {
    let mut headers = Headers::new();
    headers.insert("content-type", "text/html");
    headers.insert("content-length", html.len().to_string());
    Response { status, headers, body: html.as_bytes().to_vec()}
}