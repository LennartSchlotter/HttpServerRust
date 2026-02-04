//! # HTTP Server
//! 
//! A lightweight, modular HTTP server toolkit.
//! 
//! This crate provides parsers and request / response types to handle HTTP requests.
//! 
//! Refer to the included binary example for a complete server implementation.
pub mod request;
pub mod headers;
pub mod request_line;
pub mod server;
pub mod response;