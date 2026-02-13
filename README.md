# Rust HTTP Server
Standalone Rust Program, serving as a learning project. The goal is to implement a HTTP/1.1 Server in Rust to get familiar with the intricacies of the language.

## Additional Files
The `/tmp` folder includes additional files of certain sub-goals achieved in the process of creating the server. They showcase the learning steps in a more detailed way.

## Documentation
The code features at times extensive in-line comments to document issues, lessons learned as well as possibilities for extending the code.

## Current Implementation
- HTTP Server
    - Request Parser
    - Response Handling

## Post-Mortem
### What would I do differently?
- Have a clearer structure of the goal from the get-go to not have to refactor code and keep a cleaner separation of concerns
- Be more mindful of Security

### Mistakes made

### Fundamental Concepts learned / applied
- Network Programming
    - Protocols
        - HTTP/1.1, TCP, UDP
- General Concepts
    - Concurrency
- Rust
    - Standard Library, Ownership

## Expansion (Version 2) => Performance
- Concurrency Model Change (Tokio Async)
- `keep-alive` header

## Expansion (Version 3) => Security
- Request Timeout
- Request Size Limit
- Header Validation
- HTTP (TLS / SSL)

## Expansion (Version 4) => Routing
- Structure the routing with a simple router

## How could this be advanced further?
- HTTP Pipelining