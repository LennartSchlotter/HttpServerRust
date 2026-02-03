use std::io::{Read};

use thiserror::Error;

use crate::{headers::headers::{Headers}, request_line::request_line::{RequestLine, parse_request_line}};

/// Representation of a HTTP request with request line, headers and body
/// 
/// Includes a parse state to keep track of the progress of the parsing
#[derive(Debug)]
pub struct Request {
    /// The state of the parser.
    pub parse_state: ParseState,
    /// A custom struct representing the request line.
    pub request_line: RequestLine,
    /// A custom struct representing a list of headers.
    pub headers: Headers,
    /// The response body (can be empty).
    pub body: Vec<u8>,
}

/// Represents the different stages of the parser.
#[derive(Debug, PartialEq)]
pub enum ParseState {
    /// The parser was initialized.
    Initialized,
    /// The parser is parsing headers.
    RequestStateParsingHeaders,
    /// The parser is parsing the body.
    ParseBody,
    /// The parser finished parsing.
    Done,
}

/// Represents the kind of error that can occur during response parsing
#[derive(Error, Debug)]
pub enum HttpError {
    /// The request contains an unsupported / invalid HTTP version
    #[error("unsupported HTTP version: {0}")]
    UnsupportedVersion(String),

    /// The request contains an unsupported / invalid HTTP method.
    #[error("unsupported HTTP method: {0}")]
    InvalidMethod(String),

    /// The parser is in an invalid state.
    #[error("parser is in an invalid state")]
    InvalidParserState,

    /// The request line does not follow the RFC standard.
    #[error("request line is malformed")]
    MalformedRequestLine,

    /// The header does not follow the RFC standard.
    #[error("header is malformed")]
    MalformedHeader,

    /// The parser unexpectedly reached an end of file.
    #[error("unexpected end of file")]
    UnexpectedEOF,

    /// The passed body length does not match the header specification.
    #[error("body length does not match header")]
    InvalidBodyLength,

    /// There was a generic IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// There was an error parsing an integer to a string.
    #[error("Parsing error: {0}")]
    ParseError(#[from] std::num::ParseIntError),
}

/// Parses the contents of a reader to a Request
/// 
/// The reader may be of any type that implements `Read`
/// 
/// Throws a HttpError if the request was not valid.
pub fn request_from_reader<R: Read>(reader: &mut R) -> Result<Request, HttpError> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut temp = [0u8; 64];
    let request_line = RequestLine {method: String::new(), request_target: String::new(), http_version: String::new()};
    let headers = Headers::new();
    let body = Vec::new();
    let mut request = Request {parse_state: ParseState::Initialized, request_line, headers, body};
    let mut bytes_read = 0;

    loop{
        match request.parse_state {
            ParseState::Done => return Ok(request),
            ParseState::Initialized | ParseState::RequestStateParsingHeaders | ParseState::ParseBody => {
                let parsed = request.parse(&buffer[..bytes_read])?;
                if parsed > 0 {
                    buffer.drain(0..parsed);
                    bytes_read -= parsed;
                    continue;
                }

                if matches!(request.parse_state, ParseState::Done) {
                    return Ok(request);
                }

                let read = reader.read(&mut temp[0..])?;
                if read == 0 {
                    if matches!(request.parse_state, ParseState::Done) {
                        return Ok(request);
                    } else {
                        return Err(HttpError::UnexpectedEOF);
                    }
                }

                buffer.extend_from_slice(&temp[0..read]);
                bytes_read += read;
            }
        };
    }

}

impl Request {
    
    /// Parses passed byte data.
    /// 
    /// Returns the size of the parsed data.
    pub fn parse(&mut self, data: &[u8]) -> Result<usize, HttpError> {
        let string = String::from_utf8_lossy(data);
        let mut total_size = 0;
        match self.parse_state {
            ParseState::Initialized => {
                let (request_line_result, request_line_size) = parse_request_line(string.as_ref())?;
                if let Some(request_line) = request_line_result {
                    if request_line.http_version != "1.1" {
                        return Err(HttpError::UnsupportedVersion(request_line.http_version.to_string()))
                    }
                    self.parse_state = ParseState::RequestStateParsingHeaders;
                    self.request_line = request_line;
                }
                total_size = request_line_size;
                Ok(total_size)
            },
            ParseState::RequestStateParsingHeaders => {
                let (header_size, done) = self.headers.parse_header(string.as_bytes())?;
                total_size += header_size;
                if done {
                    self.parse_state = ParseState::ParseBody;
                }
                Ok(total_size)
            },
            ParseState::ParseBody => {
                let content = match self.headers.get("content-length") {
                    Some(value) => value,
                    None => {
                        self.parse_state = ParseState::Done;
                        return Ok(total_size);
                    }
                };

                let content_length: usize = content.parse()?;

                let already_received = self.body.len();
                if already_received > content_length {
                    return Err(HttpError::InvalidBodyLength);
                }

                let remaining = content_length.saturating_sub(self.body.len());
                let to_take = remaining.min(data.len());

                if to_take < data.len() {
                    return Err(HttpError::InvalidBodyLength);
                }

                self.body.extend_from_slice(&data[..to_take]);

                if self.body.len() < content_length {
                    return Ok(to_take);
                }

                self.parse_state = ParseState::Done;
                Ok(to_take)
            },
            ParseState::Done => {
                if !data.is_empty() {
                    return Err(HttpError::InvalidBodyLength);
                }
                Ok(0)
            }
        }
    }
}
