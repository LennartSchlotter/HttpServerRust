//! # HTTP Server
//! 
//! A lightweight, modular HTTP server toolkit.
//! 
//! This crate provides parsers and request / response types to handle HTTP requests.
//! 
//! Refer to the included binary example for a complete server implementation.
/// Logic containing parsing the HTTP.
pub mod http;
/// Logic handling runtime logic for a server instance.
pub mod runtime;