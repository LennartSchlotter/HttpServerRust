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

    //TODO This can only parse one at a time. We only take the first split.
    //FIXME Ideally we could try to recognize multiple \r\ns in here, EXCEPT for if they come right back to back as that would indicate the end of the headers.
    //Performance is negligible, but the issue is always returning to the main loop, where .drain() is called multiple times.
    ///////////////
    /// Parses passed data from a byte array to a header.
    /// 
    /// Returns the amount of data parsed to handle cases where the array contains incomplete data that should be kept.
    /// 
    pub fn parse_header<B>(&mut self, data: B) -> Result<(usize, bool),HttpError> where B: AsRef<[u8]> {
        
        // size of \r\n fixed as 2
        const CRLF_LEN: usize = 2;
        let string = String::from_utf8_lossy(&data.as_ref()[..]);

        let index = match string.find("\r\n") {
            None => return Ok((0, false)),
            Some(0) => return Ok((2, true)), //If there is a \r\n immediately on index 0, we reached the final linebreak separating headers from body
            Some(idx) => idx,
        };

        let mut split = string.split("\r\n");
        let line = split.next().unwrap(); //We can safely unwrap as split will always return at least one
        
        //Trim any optional whitespaces and split the remainders on the colon. 
        let trim = line.trim();
        let result = trim.split_once(':').ok_or(HttpError::MalformedHeader);
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

        let line_length = index + CRLF_LEN;

        return Ok((line_length, false));
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