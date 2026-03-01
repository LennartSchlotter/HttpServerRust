# Rust HTTP Server
Standalone Rust Program, serving as a learning project. The goal is to implement a HTTP/1.1 Server in Rust to get familiar with the intricacies of the language.

## Features
- Supports HTTP/1.1
- Request Parser
- Response Handling
- Full Tokio compatibility
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

# Additional Files
The `/tmp` folder includes additional files of certain sub-goals achieved in the process of creating the server. They showcase the learning steps in a more detailed way.

### Documentation
The code features at times extensive in-line comments to document issues, lessons learned as well as possibilities for extending the code.

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
    [- Dynamic path segments]
- Configuration of hardcoded values with config support
- Redirect HTTP to HTTPS

#### Known Limitation
- The chunked encoding functionality is not supported by the server implementation.

## Post-Mortem
### What would I do differently?
- Have a clearer structure of the goal from the get-go
    - Internal Code Structure and refactoring needs
    - Plan for the usage of the application
- Be more mindful of Security

### Review

### Future
- HTTP Pipelining
- Better Header handling