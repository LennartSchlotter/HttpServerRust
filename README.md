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

# Learning Process
## Fundamental Concepts learned / applied
- Network Programming
    - Protocols
        - HTTP/1.1, TCP, UDP, TLS
- General Concepts
    - Concurrency
- Rust
    - This project served as a complete introduction to the language

## Expanding from the base version
### Performance
- Concurrency Model Change (Tokio Async)
- `keep-alive` header

### Security
- Request Timeout
- Request Size Limit
- Header Validation
- HTTPS (TLS 1.2 / 1.3)

### Extensibility
- Enable easier configuration of endpoints and responses through a router
    - Streamline response generation
- Configuration of hardcoded values with config support
- Redirect HTTP to HTTPS

#### Known Limitations
- The chunked encoding functionality is not supported by the server implementation
- No method-based routing

## Post-Mortem
### What would I do differently?
- Have a clearer structure of the goal from the get-go
    - Internal Code Structure and refactoring needs
    - Plan for the usage of the application
- Be more mindful of Security

### Review
- `from_utf8_lossy` was used where a hard rejection of non-ASCII would have been more secure
- Error type grew to include concerns unrelated to HTTP parsing. A more detailed plan for module boundaries and error handling would've been preferable

### Future
- HTTP Pipelining
- Better Header handling
- Dynamic Path Segments