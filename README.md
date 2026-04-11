# Rust HTTP Server
Standalone Rust Program, serving as a learning project. The goal is to implement a HTTP/1.1 Server in Rust to get familiar with the intricacies of the language.

## Features
- Supports HTTP/1.1
- Request Parser
- Response Handling
- Async I/O via Tokio
- Keep-alive and slow requests handling
- TLS / HTTP redirect

## Usage
Code:
```rust
#[tokio::main]
async fn main() -> Result<(), HttpError> {
    //Configure router
    let router = Router::new();

    //Set up config
    let config = build_config()?;

    //Serve the application
    let _server = serve(config, router).await?;

    //Enable
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

Refer to `/examples` for additional, more detailed code that showcases functionality.
