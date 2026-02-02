use std::{io::Write};

use crate::{request::request::{HttpError, Request}, response::response::Response};

pub trait Handler {
    fn call<W: Write>(&self, req: &Request, stream: &mut W) -> Result<Option<Response>, HttpError>;
}
