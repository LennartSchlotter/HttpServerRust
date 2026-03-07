use std::{collections::HashMap, pin::Pin};

use crate::http::{
    request::{HttpError, Request},
    response::{Response, StatusCode, html_response},
};

/// A custom type boxing the Future returned by an async closure to enable storing it in the router.
type HandlerFn =
    Box<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>;

/// The Router of the application, implemented using a `HashMap` of endpoint / closure pairs.
pub struct Router(HashMap<String, HandlerFn>);

impl Router {
    /// Creates and returns a new `HashMap` representing the Router
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Registers a new route for the router.
    pub fn route<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.0.insert(
            path.to_string(),
            Box::new(move |req| Box::pin(handler(req))),
        );
    }

    /// Retrieves an optional closure if the passed endpoint is present in the router.
    #[must_use]
    fn retrieve(&self, endpoint: &str) -> Option<&HandlerFn> {
        self.0.get(endpoint)
    }

    /// Determines what happens to a given request.
    ///
    /// # Errors
    /// Throws an `HttpError` if processing the request fails.
    pub async fn call(&self, request: Request) -> Result<Response, HttpError> {
        let endpoint = request.request_line.request_target.as_str();
        let closure: Option<&HandlerFn> = self.retrieve(endpoint);
        let response = if let Some(closure) = closure {
            let result = closure(request);
            result.await
        } else {
            let body = "<html><body><h1>Not Found</h1></body></html>";
            html_response(StatusCode::NotFound, body)
        };
        Ok(response)
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Router {
    /// Prints a placeholder field for the router, as print debugging a closure is not feasible.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<closure>")
    }
}
