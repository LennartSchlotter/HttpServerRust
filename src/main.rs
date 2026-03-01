//! # Rust HTTP Server
//!
//! This binary crate provides a HTTP server implementation built on top of the library in this crate
//!
//! It supports basic request parsing and response generation.
//!
//! Refer to the library documentation of reusable components.
use httpserver::{
    http::request::HttpError,
    runtime::{
        router::Router,
        server::{build_config, serve},
    },
};

#[tokio::main]
async fn main() -> Result<(), HttpError> {
    let router = Router::new();

    let config = build_config()?;

    let _server = serve(config, router).await?;
    tokio::signal::ctrl_c().await?;
    Ok(())
}
