use tokio::io::AsyncWrite;

use crate::http::{
    request::{HttpError, Request},
    response::Response,
};

/// A trait that determines the handling for each server.
pub trait Handler: Send + Sync {
    /// Determines what happens to a given request.
    ///
    /// # Errors
    /// Throws an `HttpError` if processing the request fails.
    fn call<W: AsyncWrite + Unpin + Send>(
        &self,
        req: &Request,
        stream: W,
    ) -> impl Future<Output = Result<Option<Response>, HttpError>> + Send;
}
