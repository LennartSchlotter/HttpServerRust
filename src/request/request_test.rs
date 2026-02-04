#[cfg(test)]
mod tests {
    use std::io::{self, BufReader};

    use crate::request::request::{HttpError, request_from_reader};

    pub struct ChunkReader<'a> {
        data: &'a str,
        num_bytes_per_read: usize,
        pos: usize,
    }

    impl<'a> ChunkReader<'a> {
        pub fn new(data: &'a str, num_bytes_per_read: usize) -> Self {
            Self {
                data,
                num_bytes_per_read: num_bytes_per_read.max(1),
                pos: 0,
            }
        }
    }

    impl io::Read for ChunkReader<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.pos >= self.data.len() {
                return Ok(0)
            }

            let remaining = self.data.len() - self.pos;
            let to_read = remaining.min(self.num_bytes_per_read).min(buffer.len());

            let src = self.data.as_bytes();
            buffer[..to_read].copy_from_slice(&src[self.pos..self.pos + to_read]);
            self.pos += to_read;

            Ok(to_read)
        }
    }

    #[test]
    fn get_request_line_valid() {
        let input = "GET / HTTP/1.1\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";
        let mut chunk_reader = ChunkReader::new(input, 7);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).unwrap();

        assert_eq!(r.request_line.method, "GET");
        assert_eq!(r.request_line.request_target, "/");
        assert_eq!(r.request_line.http_version, "1.1");
    }

    #[test]
    fn get_request_line_with_path_valid() {
        let input = "GET /coffee HTTP/1.1\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";

        let mut chunk_reader = ChunkReader::new(input, input.len());
        let mut buffered = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).unwrap();

        assert_eq!(r.request_line.method, "GET");
        assert_eq!(r.request_line.request_target, "/coffee");
        assert_eq!(r.request_line.http_version, "1.1");
    }

    #[test]
    fn post_request_with_path_valid() {
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
        let r = request_from_reader(&mut buffered).unwrap();

        assert_eq!(r.request_line.method, "POST");
        assert_eq!(r.request_line.request_target, "/coffee");
        assert_eq!(r.request_line.http_version, "1.1");
    }

    #[test]
    fn invalid_number_of_requestline_parts_should_throw_malformedrequestline() {
        let input = "/coffee HTTP/1.1\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";

        let mut chunk_reader = ChunkReader::new(input, 1);
        let mut buffered = BufReader::new(&mut chunk_reader);
        let result = request_from_reader(&mut buffered);

        assert!(
            matches!(result, Err(HttpError::MalformedRequestLine)),
            "Expected Err(HttpError::MalformedRequestLine), got {result:?}"
        );
    }

    #[test]
    fn invalid_http_version_should_throw_unsupportedversion() {
        let input = "GET / HTTP/1.2\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";

        let mut chunk_reader = ChunkReader::new(input, 8);
        let mut buffered = BufReader::new(&mut chunk_reader);
        let result = request_from_reader(&mut buffered);

        assert!(
            matches!(result, Err(HttpError::UnsupportedVersion(_))),
            "Expected Err(HttpError::UnsupportedVersion), got {result:?}"
        );
    }

    #[test]
    fn invalid_request_line_order_should_throw_malformedrequestline() {
        let input = "HTTP/1.1 / GET\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";

        let mut chunk_reader = ChunkReader::new(input, 15);
        let mut buffered = BufReader::new(&mut chunk_reader);
        let result = request_from_reader(&mut buffered);

        assert!(
            matches!(result, Err(HttpError::MalformedRequestLine)),
            "Expected Err(HttpError::MalformedRequestLine), got {result:?}"
        );
    }

    #[test]
    fn invalid_http_method_should_throw_invalidmethod() {
        let input = "STOPS / HTTP/1.1\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";

        let mut chunk_reader = ChunkReader::new(input, 15);
        let mut buffered = BufReader::new(&mut chunk_reader);
        let result = request_from_reader(&mut buffered);

        assert!(
            matches!(result, Err(HttpError::InvalidMethod(_))),
            "Expected Err(HttpError::InvalidMethod), got {result:?}"
        );
    }


    #[test]
    fn request_with_extra_spaces_should_throw_malformedrequestline() {
        let input = "GET  /  HTTP/1.1\r\n\
            Host: localhost:8080\r\n\
            User-Agent: curl/7.81.0\r\n\
            Accept: */*\r\n\
            \r\n";

        let mut chunk_reader = ChunkReader::new(input, 15);
        let mut buffered = BufReader::new(&mut chunk_reader);
        let result = request_from_reader(&mut buffered);

        assert!(
            matches!(result, Err(HttpError::MalformedRequestLine)),
            "Expected Err(HttpError::MalformedRequestLine), got {result:?}"
        );
    }

    #[test]
    fn incomplete_request_should_throw_unexpectedeof() {
        let input = "GET / HTTP/1.1";
        let mut reader = input.as_bytes();

        let result = request_from_reader(&mut reader);
        
        assert!(matches!(result, Err(HttpError::UnexpectedEOF)));
    }

    #[test]
    fn valid_headers() {
        let input = "GET / HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n";
        let mut chunk_reader = ChunkReader::new(input, 7);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let mut r = request_from_reader(&mut buffered).unwrap();

        assert!(r.headers.get("host").is_some());
        assert!(r.headers.get("user-agent").is_some());
        assert!(r.headers.get("accept").is_some());
        assert_eq!(r.headers.get("host").unwrap(), "localhost:8080");
        assert_eq!(r.headers.get("user-agent").unwrap(), "curl/7.81.0");
        assert_eq!(r.headers.get("accept").unwrap(), "*/*");
    }

    #[test]
    fn request_with_malformed_headers_throws_malformedheader() {
        let input = "GET / HTTP/1.1\r\nHost localhost:8080\r\n\r\n";
        let mut chunk_reader = ChunkReader::new(input, 7);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered);

        assert!(r.is_err());
        assert!(matches!(r, Err(HttpError::MalformedHeader)));
    }




    ///////////////////////// BODY TESTS /////////////////////////////////////////////////////////



    #[test]
    fn body_valid() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        Content-Length: 12\r\n\
                        \r\n\
                        hello world!";
        
        let mut chunk_reader = ChunkReader::new(input,32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered).unwrap();
        
        assert_eq!(String::from_utf8(r.body).unwrap(), "hello world!");
    }

    #[test]
    fn body_shorter_than_content_length_should_throw_unexpectedeof() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        Content-Length: 20\r\n\
                        \r\n\
                        hello world!";

        let mut chunk_reader = ChunkReader::new(input,32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered);

        assert!(r.is_err());
        assert!(matches!(r, Err(HttpError::UnexpectedEOF)));
    }

    #[test]
    fn empty_body_with_empty_content_length_valid() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        Content-Length: 0\r\n\
                        \r\n\
                        ";

        let mut chunk_reader = ChunkReader::new(input,32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered);
        
        assert!(r.is_ok());
        let request = r.unwrap();
        assert!(request.body.is_empty());
    }

    #[test]
    fn empty_body_without_content_length_valid() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        \r\n\
                        ";

        let mut chunk_reader = ChunkReader::new(input,32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered);
        
        assert!(r.is_ok());
        let request = r.unwrap();
        assert!(request.body.is_empty());
    }

    #[test]
    fn body_longer_than_content_length_should_throw_invalidbodylength() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        Content-Length: 5\r\n\
                        \r\n\
                        hello world!";

        let mut chunk_reader = ChunkReader::new(input,30);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered);
        
        assert!(r.is_err());
        assert!(matches!(r, Err(HttpError::InvalidBodyLength)));
    }

    #[test]
    fn no_content_length_but_body_exists_valid() {
        let input = "\
            POST /st HTTP/1.1\r\n\
                        Host: localhost:8080\r\n\
                        \r\n\
                        hello world!";

        let mut chunk_reader = ChunkReader::new(input,32);
        let mut buffered: BufReader<&mut ChunkReader<'_>> = BufReader::new(&mut chunk_reader);
        let r = request_from_reader(&mut buffered);

        assert!(r.is_ok());
        let request = r.unwrap();
        assert!(request.body.is_empty());
    }
}