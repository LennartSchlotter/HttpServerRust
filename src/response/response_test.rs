#[cfg(test)]
mod tests {
    use crate::{headers::headers::Headers, response::response::{StatusCode, write_chunked_body, write_final_body_chunk, write_headers, write_status_line}};

    #[test]
    fn reason_phrase_converts_method_to_string() {
        let valid_methods = [
        (StatusCode::Ok, "OK"),
        (StatusCode::Created, "Created"),
        (StatusCode::BadRequest, "Bad Request"), 
        (StatusCode::NotFound, "Not Found"),
        (StatusCode::InternalServerError, "Internal Server Error"),
        ];

        for (method, expected) in valid_methods {
            assert_eq!(method.reason_phrase(), expected);
        }
    }

    #[test]
    fn write_status_line_produces_correct_http_line() {
        let mut buffer = Vec::new();
        let expected = "HTTP/1.1 200 OK\r\n".as_bytes();

        write_status_line(&mut buffer, StatusCode::Ok).unwrap();
        
        assert_eq!(buffer, expected);
    }

    #[test]
    fn write_headers_produces_correct_headers() {
        let mut buffer = Vec::new();
        let mut headers = Headers::new();
        headers.insert("host", "localhost:8080");
        let expected = "host: localhost:8080\r\n\r\n".as_bytes();

        write_headers(&mut buffer, &mut headers).unwrap();
        
        assert_eq!(buffer, expected);
    }

    #[test]
    fn write_chunked_bodies_formats_body() {
        let mut buffer = Vec::new();
        let data = "Let us see what happens".as_bytes();
        let expected = 
        "17\r\n\
        Let us see what happens\r\n\
        ";

        write_chunked_body(&mut buffer, data).unwrap();

        assert_eq!(buffer, expected.as_bytes());
    }

    #[test]
    fn write_final_body_chunk_formats_ending_without_trailer() {
        let mut buffer = Vec::new();
        let expected = 
        "0\r\n\
        \r\n\
        ";

        write_final_body_chunk(&mut buffer, None).unwrap();

        assert_eq!(buffer, expected.as_bytes());
    }

    #[test]
    fn write_final_body_chunk_formats_ending_with_trailer() {
        let mut buffer = Vec::new();
        let mut trailers = Headers::new();
        trailers.insert("Server-Timing", "custom-metric;dur=123.4");
        let expected = 
        "0\r\n\
        server-timing: custom-metric;dur=123.4\r\n\
        \r\n\
        ";

        write_final_body_chunk(&mut buffer, Some(trailers)).unwrap();

        assert_eq!(buffer, expected.as_bytes());
    }

}