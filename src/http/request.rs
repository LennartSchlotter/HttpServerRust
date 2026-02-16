use std::time::Duration;

use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    task::JoinError,
    time::timeout,
};

use crate::{
    http::headers::Headers,
    http::request_line::{RequestLine, parse_request_line},
};

/// Representation of a HTTP request with request line, headers and body
///
/// Includes a parse state to keep track of the progress of the parsing
#[derive(Debug)]
pub struct Request {
    /// The state of the parser.
    parse_state: ParseState,
    /// A custom struct representing the request line.
    pub request_line: RequestLine,
    /// A custom struct representing a list of headers.
    pub headers: Headers,
    /// The response body (can be empty).
    pub body: Vec<u8>,
}

/// Represents the different stages of the parser.
#[derive(Debug, PartialEq, Eq)]
enum ParseState {
    /// The parser was initialized.
    Initialized,
    /// The parser is parsing headers.
    ParseHeaders,
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

    /// An internal invariant was violated.
    /// This is most likely used as a safety net to catch errors that logically should not be able to happen.
    #[error("Internal invariant violated")]
    InternalInvariantViolated,

    /// The request to the server from the proxy failed.
    #[error("upstream request failed: {0}")]
    UpstreamRequestFailed(#[from] reqwest::Error),

    /// A blocking task failed ot complete.
    #[error("Blocking task failed: {0}")]
    TaskJoin(#[from] JoinError),

    /// The request timed out
    /// This can happen both due to the request arriving too slowly (fault of the client) and the response taking too long to arrive (fault of the server)
    #[error("Timed out")]
    Timeout,

    /// The content of the request is too large.
    /// This can happen both due to the total request size exceeding 8 MiB, but also the headers themselves exceeding 32 KiB.
    #[error("Content too large")]
    ContentTooLarge,
}

/// Parses the contents of a reader to a Request
///
/// The reader may be of any type that implements `Read`
///
/// # Errors
///
/// Throws a `HttpError` if the request was not valid.
///
/// This is related to the parsed data from the buffer containing RFC-incompatible formatting.
pub async fn request_from_reader<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Request, HttpError> {
    const READ_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
    const MAX_REQUEST_SIZE: usize = 8 * 1024 * 1024;
    const MAX_HEADER_SIZE: usize = 32 * 1024;
    let mut buffer: Vec<u8> = Vec::new();
    let mut temp = [0u8; 64];
    let request_line = RequestLine {
        method: String::new(),
        request_target: String::new(),
        http_version: String::new(),
    };
    let headers = Headers::new();
    let body = Vec::new();
    let mut request = Request {
        parse_state: ParseState::Initialized,
        request_line,
        headers,
        body,
    };
    let mut bytes_read = 0;
    let mut total_bytes_read = 0;
    let mut header_bytes_read = 0;

    loop {
        let result = timeout(READ_REQUEST_TIMEOUT, async {
            match request.parse_state {
                ParseState::Done => return Ok(true),
                ParseState::Initialized | ParseState::ParseHeaders | ParseState::ParseBody => {
                    let parsed = request.parse(&buffer[..bytes_read])?;
                    if parsed > 0 {
                        buffer.drain(0..parsed);
                        bytes_read -= parsed;
                        return Ok(false);
                    }

                    if matches!(request.parse_state, ParseState::Done) {
                        return Ok(true);
                    }

                    let read = reader.read(&mut temp[0..]).await?;
                    if read == 0 {
                        if matches!(request.parse_state, ParseState::Done) {
                            return Ok(true);
                        }
                        return Err(HttpError::UnexpectedEOF);
                    }

                    if matches!(request.parse_state, ParseState::ParseHeaders) {
                        header_bytes_read += read;
                    }

                    buffer.extend_from_slice(&temp[0..read]);
                    bytes_read += read;
                    total_bytes_read += read;

                    if total_bytes_read > MAX_REQUEST_SIZE {
                        return Err(HttpError::ContentTooLarge);
                    }

                    if header_bytes_read > MAX_HEADER_SIZE {
                        return Err(HttpError::ContentTooLarge);
                    }
                }
            }
            Ok(false)
        })
        .await;

        match result {
            Ok(Ok(true)) => return Ok(request),
            Ok(Ok(false)) => {}
            Err(_) => return Err(HttpError::Timeout),
            Ok(Err(e)) => return Err(e),
        }
    }
}

impl Request {
    /// Parses passed byte data.
    ///
    /// Returns the size of the parsed data.
    ///
    /// # Errors
    ///
    /// Throws an `HttpError` if the parsing fails.
    ///
    /// This is related to the parsed data from the buffer containing RFC-incompatible formatting.
    fn parse(&mut self, data: &[u8]) -> Result<usize, HttpError> {
        let string = String::from_utf8_lossy(data);
        let mut total_size = 0;
        match self.parse_state {
            ParseState::Initialized => {
                let (request_line_result, request_line_size) = parse_request_line(string.as_ref())?;
                if let Some(request_line) = request_line_result {
                    if request_line.http_version != "1.1" {
                        return Err(HttpError::UnsupportedVersion(request_line.http_version));
                    }
                    self.parse_state = ParseState::ParseHeaders;
                    self.request_line = request_line;
                }
                total_size = request_line_size;
                Ok(total_size)
            }
            ParseState::ParseHeaders => {
                let (header_size, done) = self.headers.parse_header(string.as_bytes())?;
                total_size += header_size;
                if done {
                    self.parse_state = ParseState::ParseBody;
                }
                Ok(total_size)
            }
            ParseState::ParseBody => {
                let Some(content) = self.headers.get("content-length") else {
                    self.parse_state = ParseState::Done;
                    return Ok(total_size);
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
            }
            ParseState::Done => {
                if !data.is_empty() {
                    return Err(HttpError::InvalidBodyLength);
                }
                Ok(0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        pin::Pin,
        task::{Context, Poll},
        time::Duration,
    };

    use tokio::io::AsyncWriteExt;
    use tokio::io::{self, AsyncRead, BufReader, ReadBuf};

    use crate::http::request::{HttpError, request_from_reader};

    // Helpers

    pub struct ChunkReader<'a> {
        data: &'a [u8],
        num_bytes_per_read: usize,
        pos: usize,
    }

    impl<'a> ChunkReader<'a> {
        pub fn new(data: &'a str, num_bytes_per_read: usize) -> Self {
            Self {
                data: data.as_bytes(),
                num_bytes_per_read: num_bytes_per_read.max(1),
                pos: 0,
            }
        }
    }

    impl AsyncRead for ChunkReader<'_> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            if self.pos >= self.data.len() {
                return Poll::Ready(Ok(()));
            }

            let remaining = self.data.len() - self.pos;
            let max_take = self.num_bytes_per_read.min(remaining);
            let max_take = max_take.min(buf.remaining());

            if max_take == 0 {
                return Poll::Ready(Ok(()));
            }

            let chunk = &self.data[self.pos..self.pos + max_take];
            buf.put_slice(chunk);

            self.pos += max_take;

            Poll::Ready(Ok(()))
        }
    }

    fn large_body_test_input(size: usize) -> String {
        let mut s = String::with_capacity(size + 512);
        s.push_str("POST / HTTP/1.1\r\nContent-Length: ");
        s.push_str(&size.to_string());
        s.push_str("\r\n\r\n");
        s.extend(std::iter::repeat_n('x', size));
        s
    }

    fn large_header_test_input(size: usize) -> String {
        let mut s = String::with_capacity(size + 512);
        s.push_str("POST / HTTP/1.1\r\n");
        s.push_str("Host: example.com\r\n");
        s.push_str("Content-Length: 0\r\n");
        let line = "Test-Header: xxxxxxxxxxxxxxxxxxxxxxxxxxxx\r\n";
        let line_len = line.len();
        let count = size.div_ceil(line_len);
        for _ in 0..count {
            s.push_str(line);
        }
        s.push_str("\r\n");
        s
    }

    // Tests

    #[tokio::test]
    async fn get_request_line_valid() {
        let input = "GET / HTTP/1.1\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";
        let mut chunk_reader = ChunkReader::new(input, 7);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await.unwrap();

        assert_eq!(r.request_line.method, "GET");
        assert_eq!(r.request_line.request_target, "/");
        assert_eq!(r.request_line.http_version, "1.1");
    }

    #[tokio::test]
    async fn get_request_line_with_path_valid() {
        let input = "GET /coffee HTTP/1.1\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";

        let mut chunk_reader = ChunkReader::new(input, input.len());
        let mut buffered = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await.unwrap();

        assert_eq!(r.request_line.method, "GET");
        assert_eq!(r.request_line.request_target, "/coffee");
        assert_eq!(r.request_line.http_version, "1.1");
    }

    #[tokio::test]
    async fn post_request_with_path_valid() {
        let input = "POST /coffee HTTP/1.1\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             Content-Length: 17\r\n\
             Content-Type: text/plain\r\n\
             \r\n\
             flavor: dark mode";

        let mut chunk_reader = ChunkReader::new(input, 500);
        let mut buffered = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await.unwrap();

        assert_eq!(r.request_line.method, "POST");
        assert_eq!(r.request_line.request_target, "/coffee");
        assert_eq!(r.request_line.http_version, "1.1");
    }

    #[tokio::test]
    async fn invalid_number_of_requestline_parts_should_throw_malformedrequestline() {
        let input = "/coffee HTTP/1.1\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";

        let mut chunk_reader = ChunkReader::new(input, 1);
        let mut buffered = BufReader::new(&mut chunk_reader);
        let result = request_from_reader(&mut buffered).await;

        assert!(
            matches!(result, Err(HttpError::MalformedRequestLine)),
            "Expected Err(HttpError::MalformedRequestLine), got {result:?}"
        );
    }

    #[tokio::test]
    async fn invalid_http_version_should_throw_unsupportedversion() {
        let input = "GET / HTTP/1.2\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";

        let mut chunk_reader = ChunkReader::new(input, 8);
        let mut buffered = BufReader::new(&mut chunk_reader);
        let result = request_from_reader(&mut buffered).await;

        assert!(
            matches!(result, Err(HttpError::UnsupportedVersion(_))),
            "Expected Err(HttpError::UnsupportedVersion), got {result:?}"
        );
    }

    #[tokio::test]
    async fn invalid_request_line_order_should_throw_malformedrequestline() {
        let input = "HTTP/1.1 / GET\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";

        let mut chunk_reader = ChunkReader::new(input, 15);
        let mut buffered = BufReader::new(&mut chunk_reader);
        let result = request_from_reader(&mut buffered).await;

        assert!(
            matches!(result, Err(HttpError::MalformedRequestLine)),
            "Expected Err(HttpError::MalformedRequestLine), got {result:?}"
        );
    }

    #[tokio::test]
    async fn invalid_http_method_should_throw_invalidmethod() {
        let input = "STOPS / HTTP/1.1\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";

        let mut chunk_reader = ChunkReader::new(input, 15);
        let mut buffered = BufReader::new(&mut chunk_reader);
        let result = request_from_reader(&mut buffered).await;

        assert!(
            matches!(result, Err(HttpError::InvalidMethod(_))),
            "Expected Err(HttpError::InvalidMethod), got {result:?}"
        );
    }

    #[tokio::test]
    async fn request_with_extra_spaces_should_throw_malformedrequestline() {
        let input = "GET  /  HTTP/1.1\r\n\
            Host: localhost:8080\r\n\
            User-Agent: curl/7.81.0\r\n\
            Accept: */*\r\n\
            \r\n";

        let mut chunk_reader = ChunkReader::new(input, 15);
        let mut buffered = BufReader::new(&mut chunk_reader);
        let result = request_from_reader(&mut buffered).await;

        assert!(
            matches!(result, Err(HttpError::MalformedRequestLine)),
            "Expected Err(HttpError::MalformedRequestLine), got {result:?}"
        );
    }

    #[tokio::test]
    async fn incomplete_request_should_throw_unexpectedeof() {
        let input = "GET / HTTP/1.1";
        let mut reader = input.as_bytes();

        let result = request_from_reader(&mut reader).await;

        assert!(matches!(result, Err(HttpError::UnexpectedEOF)));
    }

    #[tokio::test]
    async fn valid_headers() {
        let input = "GET / HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
        let mut chunk_reader = ChunkReader::new(input, 7);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let mut r = request_from_reader(&mut buffered).await.unwrap();

        assert!(r.headers.get("host").is_some());
        assert!(r.headers.get("user-agent").is_some());
        assert!(r.headers.get("accept").is_some());
        assert_eq!(r.headers.get("host").unwrap(), "localhost:8080");
        assert_eq!(r.headers.get("user-agent").unwrap(), "curl/7.81.0");
        assert_eq!(r.headers.get("accept").unwrap(), "*/*");
    }

    #[tokio::test]
    async fn request_with_malformed_headers_throws_malformedheader() {
        let input = "GET / HTTP/1.1\r\nHost localhost:8080\r\n\r\n";
        let mut chunk_reader = ChunkReader::new(input, 7);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await;

        assert!(r.is_err());
        assert!(matches!(r, Err(HttpError::MalformedHeader)));
    }

    /// This test is a little contrived. It simulates the client never closing the connection through a lack of `client_write.drop()`
    #[tokio::test]
    async fn server_times_out_when_request_read_too_long() {
        tokio::time::pause();

        let (server_read, mut client_write) = io::duplex(4096);
        let mut buffered = BufReader::new(server_read);

        let headers = "\
                POST /st HTTP/1.1\r\n\
                Host: localhost:8080\r\n\
                Content-Length: 100000\r\n\
                \r\n";

        client_write.write_all(headers.as_bytes()).await.unwrap();
        client_write.write_all(b"abcd").await.unwrap();
        client_write.flush().await.unwrap();

        tokio::time::advance(Duration::from_millis(200)).await;
        tokio::time::advance(Duration::from_secs(31)).await; //TODO 

        let result = request_from_reader(&mut buffered).await.unwrap_err();
        assert!(matches!(result, HttpError::Timeout));
    }

    ///////////////////////// BODY TESTS /////////////////////////////////////////////////////////

    #[tokio::test]
    async fn body_valid() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        Content-Length: 12\r\n\
                        \r\n\
                        hello world!";

        let mut chunk_reader = ChunkReader::new(input, 32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await.unwrap();

        assert_eq!(String::from_utf8(r.body).unwrap(), "hello world!");
    }

    #[tokio::test]
    async fn body_shorter_than_content_length_should_throw_unexpectedeof() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        Content-Length: 20\r\n\
                        \r\n\
                        hello world!";

        let mut chunk_reader = ChunkReader::new(input, 32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await;

        assert!(r.is_err());
        assert!(matches!(r, Err(HttpError::UnexpectedEOF)));
    }

    #[tokio::test]
    async fn empty_body_with_empty_content_length_valid() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        Content-Length: 0\r\n\
                        \r\n\
                        ";

        let mut chunk_reader = ChunkReader::new(input, 32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await;

        assert!(r.is_ok());
        let request = r.unwrap();
        assert!(request.body.is_empty());
    }

    #[tokio::test]
    async fn empty_body_without_content_length_valid() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        \r\n\
                        ";

        let mut chunk_reader = ChunkReader::new(input, 32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await;

        assert!(r.is_ok());
        let request = r.unwrap();
        assert!(request.body.is_empty());
    }

    #[tokio::test]
    async fn body_longer_than_content_length_should_throw_invalidbodylength() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        Content-Length: 5\r\n\
                        \r\n\
                        hello world!";

        let mut chunk_reader = ChunkReader::new(input, 30);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await;

        assert!(r.is_err());
        assert!(matches!(r, Err(HttpError::InvalidBodyLength)));
    }

    #[tokio::test]
    async fn no_content_length_but_body_exists_valid() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        \r\n\
                        hello world!";

        let mut chunk_reader = ChunkReader::new(input, 32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await;

        assert!(r.is_ok());
        let request = r.unwrap();
        assert!(request.body.is_empty());
    }

    #[tokio::test]
    async fn rejects_when_body_exceeds_limit() {
        let input = large_body_test_input(8 * 1024 * 1024);

        let mut chunk_reader = ChunkReader::new(&input, 32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await;

        assert!(matches!(r, Err(HttpError::ContentTooLarge)));
    }

    #[tokio::test]
    async fn reject_when_headers_exceed_limit() {
        let input = large_header_test_input(32 * 1024);

        let mut chunk_reader = ChunkReader::new(&input, 32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).await;

        assert!(matches!(r, Err(HttpError::ContentTooLarge)));
    }
}
