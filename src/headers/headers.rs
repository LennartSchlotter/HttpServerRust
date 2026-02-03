use std::{collections::HashMap};

use crate::request::request::HttpError;

/// A HashMap of two strings representing key, value pairs used in HTTP Headers.
/// 
/// Hash Maps do not guarantee ordering in Rust. SHOULD be fine as Http Headers / Trailers do not need to be ordered
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Headers(HashMap<String, String>);

impl Headers {

    /// Returns a new HashMap constructed as a 'Headers' struct
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Inserts a new entry into the Headers struct by passing both key and value
    /// 
    /// # Examples
    /// ```
    /// let mut headers = httpserver::headers::headers::Headers::new();
    /// headers.insert("drink", "milk");
    /// ```
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>){
        self.0.insert(key.into(), value.into());
    }

    /// Retrieves the value of a specified key.
    /// 
    /// Returns None if the specified key was not found in the header.
    /// 
    /// # Examples
    /// ```
    /// let mut headers = httpserver::headers::headers::Headers::new();
    /// headers.insert("drink", "milk");
    /// assert_eq!(headers.get("drink"), Some("milk"));
    /// ```
    pub fn get(&mut self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    /// Appends a key / value pair into the Header.
    /// 
    /// # Examples
    /// ```
    /// let mut headers = httpserver::headers::headers::Headers::new();
    /// headers.insert("drink", "milk");
    /// headers.append("drink", "water");
    /// headers.append("food", "pizza");
    /// assert_eq!(headers.get("drink"), Some("milk, water"));
    /// assert_eq!(headers.get("food"), Some("pizza"));
    /// ```
    pub fn append(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();

        if let Some(existing) = self.0.get_mut(&key) {
            if !existing.is_empty() {
                existing.push_str(", ");
            }
            existing.push_str(&value);
        } else {
            self.0.insert(key, value);
        }
    }

    /// Implements an iterator for the Header
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.0.iter().map(|(key, value)| (key.as_str(), value.as_str()))
    }

    /// Parses passed data from a byte array to a header.
    /// 
    /// Returns the amount of data parsed to handle cases where the array contains incomplete data that should be kept.
    /// 
    pub fn parse_header<B>(&mut self, data: B) -> Result<(usize, bool),HttpError> where B: AsRef<[u8]> {

        // size of \r\n fixed as 2
        const CRLF_LEN: usize = 2;
        let string = String::from_utf8_lossy(&data.as_ref()[..]);
        let mut line_length = 0;

        if string.find("\r\n\r\n").is_some() {
            let headers = string.split("\r\n");
            for header in headers {
                if header.is_empty() {
                    line_length += CRLF_LEN; //There is still one linebreak left here, the one separating headers from body
                    break;
                }
                line_length += header.len() + CRLF_LEN;
                self.create_header_from_string(header)?;
            }
            return Ok((line_length, true));
        }

        if string.find("\r\n").is_some() {
            if let Some((base, _end)) = string.rsplit_once("\r\n") {
                for line in base.split("\r\n") {
                    if line.is_empty() {
                        line_length += CRLF_LEN; //There is still one linebreak left here, the one separating headers from body
                        return Ok((line_length, true));
                    }
                    line_length += line.len() + CRLF_LEN;
                    self.create_header_from_string(line)?;
                }
                return Ok((line_length, false));
            }
        }

        return Ok((0, false));
    }

    fn create_header_from_string(&mut self, string: &str) -> Result<(), HttpError> {
        let trim = string.trim();
        let result = trim.split_once(":").ok_or(HttpError::MalformedHeader);
        let (key, mut value) = result.unwrap();
        value = value.trim();

        if key.find(" ").is_some() {
            return Err(HttpError::MalformedHeader);
        }

        if !key.chars().all(is_valid_char) {
            return Err(HttpError::MalformedHeader);
        }

        let key_lowercase = key.to_lowercase();
        
        if self.0.contains_key(&key_lowercase) {
            self.append(key_lowercase, value);
        } else {
            self.insert(key_lowercase, value);
        }

        return Ok(())
    }
}

/// Helper method to determine whether the passed character is valid according to https://www.rfc-editor.org/rfc/rfc9110#section-5.6.2
fn is_valid_char(c: char) -> bool {
    if c.is_ascii_alphanumeric() {
        return true;
    }

    matches!(c,
        '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' |
        '-' | '.' | '^' | '_' | '`' | '|' | '~'
    )
}