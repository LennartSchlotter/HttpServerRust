use std::{
    fmt::Debug,
    io::Error,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    time::{sleep, timeout},
};

use crate::http::response::{write_headers, write_status_line};
use crate::http::{
    headers::Headers,
    request::{HttpError, request_from_reader},
    response::{Response, StatusCode, html_response},
};
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
                    println!("Accepted a new connection");
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
                    sleep(Duration::from_millis(50)).await;
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
    tokio::spawn(async move {
        state_for_thread.listen().await;
    });
    Ok(serverhandle)
}

/// Handles a specific connection's parsing based on the associated TCP stream.
///
/// # Errors
///
/// Throws an `HttpError` if the parsing process fails.
async fn handle<H: Handler>(mut stream: TcpStream, handler: &H) -> Result<(), HttpError> {
    const SERVER_TIMEOUT: Duration = Duration::from_secs(120);

    loop {
        let result = timeout(SERVER_TIMEOUT, process_request(&mut stream, handler)).await;

        match result {
            Ok(Ok(should_continue)) => {
                if !should_continue {
                    return Ok(());
                }
            }
            Ok(Err(_e)) => {
                break;
            }
            Err(_elapsed) => {
                let html = "<html><body><h1>Gateway Timed out</h1></body></html>";
                let response = html_response(StatusCode::GatewayTimeout, html);

                write_status_line(&mut stream, response.status).await?;
                let mut headers = response.headers;
                write_headers(&mut stream, &mut headers).await?;
                stream.write_all(&response.body).await?;
                stream.flush().await?;
                break;
            }
        }
    }
    Ok(())
}

/// Handles a singular request from the associated Tcp Stream.
///
/// # Errors
///
/// Throws an `HttpError` if parsing fails or if a timeout occurs.
async fn process_request<H: Handler>(
    mut stream: &mut TcpStream,
    handler: &H,
) -> Result<bool, HttpError> {
    const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(15);
    let request_future = request_from_reader(&mut stream);
    let request_res = timeout(KEEP_ALIVE_TIMEOUT, request_future).await;
    let mut request = match request_res {
        Ok(Ok(req)) => req,
        Ok(Err(HttpError::UnexpectedEOF)) => {
            return Ok(true);
        }
        Ok(Err(HttpError::Timeout)) => {
            let html = "<html><body><h1>Request timed out</h1></body></html>";
            let response = html_response(StatusCode::RequestTimeout, html);

            write_response(stream, response).await?;
            return Ok(false);
        }
        Ok(Err(_e)) => {
            let html = "<html><body><h1>Bad Request</h1></body></html>";
            let response = html_response(StatusCode::BadRequest, html);

            write_response(stream, response).await?;
            return Ok(false);
        }
        Err(_) => {
            let html = "<html><body><h1>Bad Request</h1></body></html>";
            let response = html_response(StatusCode::BadRequest, html);
            write_response(stream, response).await?;
            return Ok(false);
        }
    };

    // FIXME We should probably have a dedicated place to manage headers
    let keep_alive = Headers::get(&mut request.headers, "connection") != Some("close");

    let response = handler.call(&request, &mut stream).await?;
    if let Some(response) = response {
        write_status_line(&mut stream, response.status).await?;
        let mut headers = response.headers;
        write_headers(&mut stream, &mut headers).await?;
        stream.write_all(&response.body).await?;

        let connection_value = headers.get("connection");
        if connection_value == Some("close") {
            Ok(false)
        } else {
            if !keep_alive {
                return Ok(false);
            }
            stream.flush().await?;
            Ok(true)
        }
    } else {
        stream.flush().await?;
        Ok(false)
    }
}

/// Helper function to group together the write operations given a TCP Stream and a response object.
///
/// # Errors
///
/// Throws an `HttpError` if the write process fails.
async fn write_response(mut stream: &mut TcpStream, response: Response) -> Result<(), HttpError> {
    write_status_line(&mut stream, response.status).await?;
    let mut headers = response.headers;
    write_headers(&mut stream, &mut headers).await?;
    stream.write_all(&response.body).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use reqwest::Client;
    use tokio::{io::AsyncWrite, time::timeout};

    use crate::{
        http::{
            request::{HttpError, Request},
            response::{Response, StatusCode, html_response},
        },
        runtime::{handler::Handler, server::serve},
    };

    struct MyHandler;

    impl Handler for MyHandler {
        async fn call<W: AsyncWrite + Unpin + Send>(
            &self,
            request: &Request,
            _stream: W,
        ) -> Result<Option<Response>, HttpError> {
            if request.request_line.request_target.as_str() == "/yourproblem" {
                let body = "<html><body><h1>Bad Request</h1></body></html>";
                let response = html_response(StatusCode::BadRequest, body);
                Ok(Some(response))
            } else {
                let body = "<html><body><h1>All good!</h1></body></html>";
                let response = html_response(StatusCode::Ok, body);
                Ok(Some(response))
            }
        }
    }

    #[tokio::test]
    async fn server_can_establish_connection() {
        let handler = MyHandler;
        let handler_arc = Arc::new(handler);
        let server = serve(8080, handler_arc)
            .await
            .expect("Failed to start server");

        let base_url = format!("http://127.0.0.1:{}", 8080);

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        let client = client.clone();
        let url = format!("{base_url}/test");

        let task = tokio::spawn(async move {
            let resp = client.get(&url).send().await.expect("Request failed");
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            (status, text)
        });

        let result = timeout(Duration::from_secs(10), task)
            .await
            .expect("Test timed out");
        let (status, _body) = result.unwrap();
        assert!(status.is_success());
        server.close();
    }

    #[tokio::test]
    async fn endpoints_write_correct_response() {
        let handler = MyHandler;
        let handler_arc = Arc::new(handler);
        let server = serve(8081, handler_arc)
            .await
            .expect("Failed to start server");

        let base_url = format!("http://127.0.0.1:{}", 8081);

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        let client = client.clone();
        let url = format!("{base_url}/yourproblem");

        let task = tokio::spawn(async move {
            let resp = client.get(&url).send().await.expect("Request failed");
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            (status, text)
        });

        let result = timeout(Duration::from_secs(10), task)
            .await
            .expect("Test timed out");
        let (status, _body) = result.unwrap();
        assert!(status.is_client_error());
        server.close();
    }

    #[tokio::test]
    async fn server_can_establish_multiple_connections() {
        let handler = MyHandler;
        let handler_arc = Arc::new(handler);
        let server = serve(8082, handler_arc)
            .await
            .expect("Failed to start server");

        let base_url = format!("http://127.0.0.1:{}", 8082);

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        let client1 = client.clone();
        let client2 = client;
        let url = format!("{base_url}/yourproblem");

        tokio::spawn(async move {
            let resp = client1.get(&url).send().await;
            let resp2 = client2.get(&url).send().await;
            assert!(resp.is_ok());
            assert!(resp2.is_ok());
            (resp, resp2)
        });

        server.close();
    }

    #[tokio::test]
    async fn server_works_concurrently() {
        const CONCURRENT_REQUESTS: usize = 20;
        let handler = MyHandler;
        let handler_arc = Arc::new(handler);
        let server = serve(8083, handler_arc)
            .await
            .expect("Failed to start server");

        let base_url = format!("http://127.0.0.1:{}", 8083);

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        let start = std::time::Instant::now();

        let tasks: Vec<_> = (0..CONCURRENT_REQUESTS)
            .map(|_| {
                let client = client.clone();
                let url = format!("{base_url}/test");
                tokio::spawn(async move {
                    let resp = client.get(&url).send().await.expect("Request failed");
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    (status, text)
                })
            })
            .collect();

        let results = futures::future::join_all(tasks).await;
        let elapsed = start.elapsed();

        for res in results {
            let (status, _body) = res.unwrap();
            assert!(status.is_success());
        }

        // Heuristic assumption: if requests WERE processed sequentially, the time would be roughly equal to the amount * time for one
        // With concurrency it should be significantly slower, at least roughly the duration of the slowest handler
        println!("Completed {CONCURRENT_REQUESTS} requests in {elapsed:?}");
        assert!(elapsed < Duration::from_secs(1));

        server.close();
    }
}
