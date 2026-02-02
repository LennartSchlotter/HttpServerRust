#[cfg(test)]
mod tests {
    use crate::{request::request::HttpError, request_line::request_line::parse_request_line};

    #[test]
    fn get_request_line_valid() {
        let input = "GET / HTTP/1.1\r\n
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";
        let (result, result_size) = parse_request_line(input).unwrap();

        assert!(result.is_some());
        let request_line = result.unwrap();
        assert_eq!(request_line.method, "GET");
        assert_eq!(request_line.request_target, "/");
        assert_eq!(request_line.http_version, "1.1");
        assert_eq!(result_size, 16);
    }

    #[test]
    fn get_request_line_with_path_valid() {
        let input = "GET /coffee HTTP/1.1\r\n\
             Host: localhost:8080\r\n\
             User-Agent: curl/7.81.0\r\n\
             Accept: */*\r\n\
             \r\n";

        let (result, result_size) = parse_request_line(input).unwrap();

        assert!(result.is_some());
        let request_line = result.unwrap();
        assert_eq!(request_line.method, "GET");
        assert_eq!(request_line.request_target, "/coffee");
        assert_eq!(request_line.http_version, "1.1");
        assert_eq!(result_size, 22);
    }

    #[test]
    fn request_line_return_none_when_incomplete_call() {
        let input = "GET /coffee HTTP/1.";

        let (result, result_size) = parse_request_line(input).unwrap();
        assert!(result.is_none());
        assert_eq!(result_size, 0);
    }

    #[test]
    fn request_line_return_throw_malformed_when_incorrect_splitting() {
        let input = "GET/coffeeHTTP/1.1\r\n";

        let result = parse_request_line(input);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(HttpError::MalformedRequestLine)),
            "Expected Err(HttpError::MalformedRequestLine), got {result:?}"
        );
    }

    #[test]
    fn request_line_return_throw_malformed_when_wrong_http_definition() {
        let input = "GET /coffee HTT/1.1\r\n";

        let result = parse_request_line(input);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(HttpError::MalformedRequestLine)),
            "Expected Err(HttpError::MalformedRequestLine), got {result:?}"
        );
    }

    #[test]
    fn request_line_return_throw_invalid_method() {
        let input = "TAKE /coffee HTTP/1.1\r\n";

        let result = parse_request_line(input);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(HttpError::InvalidMethod(_))),
            "Expected Err(HttpError::InvalidMethod), got {result:?}"
        );
    }
}