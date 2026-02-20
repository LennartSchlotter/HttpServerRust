use crate::http::request::HttpError;

/// Handles performing the TLS Handshake via RustLS.
/// 
/// # Errors
/// 
/// Returns an `HttpError` when performing the handshake failed.
pub async fn handle_tls() -> Result<(), HttpError> {
    //We are just the Server:
    //1) Read what the client sends.
    //2) Verify what it sent: Client Hello, Protocol Version (verify correct), Client Random, List of Cipher Suites
    //0) Can be done at any point, I guess as soon as a conncetion is established: Generate Server Random
    //3) Receives client hello (incl params + cipher suites), creates the master secret (I'm still not 100% sure on what the master secret is)
    //4) Send Server Hello (incl server's cert, digital signature, server random and chosen suite)
    //5) Send Server Finished 
    //6) Wait for Client Finished.

    return Ok(());
}