use std::{
    io::Error,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{io::AsyncWriteExt, net::{TcpListener, TcpStream}};

use crate::http::request::{HttpError, request_from_reader};
use crate::http::response::{write_headers, write_status_line};
use crate::runtime::handler::Handler;

/// A struct representing an instance of a `HttpServer`, containing the state of the server.
#[derive(Debug)]
pub struct Server<H: Handler> {
    server_state: Arc<ServerState<H>>,
}

/// A struct representing the state of a server with the associated listener, whether or not the server has been closed and the handler.
#[derive(Debug)]
struct ServerState<H: Handler> {
    listener: TcpListener,
    closed: AtomicBool,
    handler: Arc<H>,
}

impl<H: Handler> Server<H> {
    /// Sets the closed state of the server it's called on.
    pub fn close(&self) {
        self.server_state.closed.store(true, Ordering::SeqCst);
    }
}

impl<H: Handler + Send + Sync + 'static> ServerState<H> {
    /// Called on a `ServerState`, listening for connections.
    pub async fn listen(self: Arc<Self>) {
        loop {
            if self.closed.load(Ordering::SeqCst) {
                println!("We cannot take any new connections so stop");
                return;
            }
            match self.listener.accept().await {
                Ok((stream, _)) => {
                    let handler_clone = Arc::clone(&self.handler);
                    tokio::spawn(async move {
                        if let Err(e) = handle(stream, &*handler_clone).await {
                            eprintln!("Encountered error handling the stream: {e}");
                        }
                    });
                }
                Err(error) => {
                    if self.closed.load(Ordering::SeqCst) {
                        break;
                    }
                    eprintln!("Encountered error accepting connection: {error:}");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
    }
}

/// Serves an instance of the Http Server based on the passed handler on the specified port
///
/// # Errors
///
/// Throws an Error if binding the tcp listener fails.
pub async fn serve<H: Handler + Send + Sync + 'static>(
    port: u16,
    handler: Arc<H>,
) -> Result<Server<H>, Error> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let state = ServerState {
        listener,
        handler,
        closed: AtomicBool::new(false),
    };
    let state_for_main = Arc::new(state);
    let state_for_thread = Arc::clone(&state_for_main);
    let serverhandle = Server {
        server_state: state_for_main,
    };
    tokio::spawn(async move {state_for_thread.listen().await;});
    Ok(serverhandle)
}

/// Handles a specific connection's parsing based on the associated TCP stream.
///
/// # Errors
///
/// Throws an `HttpError` if the parsing process fails.
async fn handle<H: Handler>(mut stream: TcpStream, handler: &H) -> Result<(), HttpError> {
    let request = request_from_reader(&mut stream).await?;
    let response = handler.call(&request, &mut stream).await?;
    match response {
        Some(response) => {
            write_status_line(&mut stream, response.status).await?;
            let mut headers = response.headers;
            write_headers(&mut stream, &mut headers).await?;
            stream.write_all(&response.body).await?;
            stream.flush().await?;
        }
        None => {
            stream.flush().await?;
        }
    }
    Ok(())
}