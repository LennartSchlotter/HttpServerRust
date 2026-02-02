#[cfg(test)]
mod tests {
    use crate::{headers::headers::Headers, request::request::HttpError};

    #[test]
    fn single_header_valid() {
        let input = "Host: localhost:8080\r\n\r\n";
        let mut headers = Headers::new();
        let result = headers.parse_header(input);
        assert!(result.is_ok());

        let (size, done) = result.unwrap();
        assert_eq!(headers.get("host").unwrap(), "localhost:8080");
        assert_eq!(size, 22);
        assert_eq!(done, false);
    }

    #[test]
    fn single_header_extra_whitespace_valid() {
        let input = "        Host: localhost:8080\r\n\r\n             ";
        let mut headers = Headers::new();
        let result = headers.parse_header(input);
        assert!(result.is_ok());

        let (size, done) = result.unwrap();
        assert_eq!(headers.get("host").unwrap(), "localhost:8080");
        assert_eq!(size, 30);
        assert_eq!(done, false);
    }

    #[test]
    fn single_header_extra_whitespace_value_valid() {
        let input = "        HoSt:    localhost:8080\r\n\r\n             ";
        let mut headers = Headers::new();
        let result = headers.parse_header(input);
        assert!(result.is_ok());

        let (size, done) = result.unwrap();
        assert_eq!(headers.get("host").unwrap(), "localhost:8080");
        assert_eq!(size, 33);
        assert_eq!(done, false);
    }

    #[test]
    fn single_header_no_whitespaces_valid() {
        let input = "Host:localhost:8080\r\n\r\n";
        let mut headers = Headers::new();
        let result = headers.parse_header(input);
        assert!(result.is_ok());

        let (size, done) = result.unwrap();
        assert_eq!(headers.get("host").unwrap(), "localhost:8080");
        assert_eq!(size, 21);
        assert_eq!(done, false);
    }

    #[test]
    fn two_headers_valid() {
        let input = "Host: localhost:8080\r\nHost:localhost:8081";
        let mut headers = Headers::new();
        let result = headers.parse_header(input);
        assert!(result.is_ok());

        let (size, done) = result.unwrap();
        assert_eq!(headers.get("host").unwrap(), "localhost:8080");
        assert_eq!(size, 22);
        assert_eq!(done, false);
    }

    #[test]
    fn invalid_spacing_headers_should_throw_malformedheader() {
        let input = "          Host : localhost:8080          \r\n\r\n";
        let mut headers = Headers::new();
        let result = headers.parse_header(input);
        assert!(matches!(result, Err(HttpError::MalformedHeader)));
    }

    #[test]
    fn it_valid_done() {
        let input = "\r\nhello123";
        let mut headers = Headers::new();
        let result = headers.parse_header(input);
        assert!(result.is_ok());

        let (size, done) = result.unwrap();
        assert_eq!(size, 2);
        assert_eq!(done, true);
    }

    #[test]
    fn incomplete_request_valid() {
        let input = "key: value";
        let mut headers = Headers::new();
        let result = headers.parse_header(input);
        assert!(result.is_ok());

        let (size, done) = result.unwrap();
        assert_eq!(size, 0);
        assert_eq!(done, false);
    }

    #[test]
    fn invalid_name_character_should_throw_error() {
        let input = "@:email\r\n";
        let mut headers = Headers::new();
        let result = headers.parse_header(input);
        assert!(result.is_err());
    }

    #[test]
    fn multple_values_valid() {
        let input = "Host: localhost:8080\r\n\r\n";
        let mut headers = Headers::new();
        headers.insert("host", "localhost:8081");
        let result = headers.parse_header(input);
        assert!(result.is_ok());

        let (size, done) = result.unwrap();
        assert_eq!(headers.get("host").unwrap(), "localhost:8081, localhost:8080");
        assert_eq!(size, 22);
        assert_eq!(done, false);
    }
}