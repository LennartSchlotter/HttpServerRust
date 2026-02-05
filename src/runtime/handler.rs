use std::io::Write;

use crate::{http::request::{HttpError, Request}, http::response::Response};

/// A trait that determines the handling for each server.
pub trait Handler {

    /// Determines what happens to a given request.
    /// 
    /// # Errors
    /// Throws an `HttpError` if processing the request fails.
    fn call<W: Write>(&self, req: &Request, stream: &mut W) -> Result<Option<Response>, HttpError>;
}
