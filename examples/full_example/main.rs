//! # Rust HTTP Server example implementation
use httpserver::{
    http::{
        request::HttpError,
        response::{Response, StatusCode, file_response, html_response},
    },
    runtime::{
        router::Router,
        server::{build_config, serve},
    },
};

#[tokio::main]
async fn main() -> Result<(), HttpError> {
    let mut router = Router::new();
    router.route("/", |_req| async {
        file_response(StatusCode::Ok, "/static/hello.html")
            .await
            .unwrap_or_else(|_| {
                html_response(
                    StatusCode::InternalServerError,
                    "<html><body><h1>Internal Server Error</h1></body></html>",
                )
            })
    });

    router.route("/echo", |req| async move {
        let body = String::from_utf8_lossy(&req.body).to_string();
        html_response(StatusCode::Ok, &format!("<html><body>{body}</body></html>"))
    });

    router.route("/api/hello", |_req| async {
        let body = r#"{"message": "hello"}"#;
        let mut headers = httpserver::http::headers::Headers::new();
        headers.insert("content-type", "application/json");
        headers.insert("content-length", body.len().to_string());
        Response {
            status: StatusCode::Ok,
            headers,
            body: body.as_bytes().to_vec(),
        }
    });

    // Example POST. Since the server doesn't differentiate by method, both GET /submit and POST /submit would work.
    router.route("/submit", |req| async move {
        if req.request_line.method != "POST" {
            return html_response(
                StatusCode::BadRequest,
                "<html><body><h1>Method Not Allowed</h1></body></html>",
            );
        }

        let Ok(_body) = String::from_utf8(req.body) else {
            return html_response(
                StatusCode::BadRequest,
                "<html><body><h1>Bad Request</h1></body></html>",
            );
        };

        html_response(
            StatusCode::Ok,
            "<html><body><h1>Received</h1></body></html>",
        )
    });

    // This automatically builds a config placed anywhere in the directory.
    // It recognizes any file called `config.toml`
    let config = build_config()?;

    // This serves the server with the configured values.
    let _server = serve(config, router).await?;

    // Ctrl+C shuts down the server through a tokio signal. This shutdown is not graceful, so any in-flight connections are dropped.
    tokio::signal::ctrl_c().await?;
    Ok(())
}
