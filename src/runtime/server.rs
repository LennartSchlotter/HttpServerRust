use crate::http::response::{write_headers, write_status_line};
use crate::http::{
    headers::Headers,
    request::{HttpError, request_from_reader},
    response::{Response, StatusCode, html_response},
};
use crate::runtime::handler::Handler;
use rustls::{
    ServerConfig,
    pki_types::{CertificateDer, PrivatePkcs8KeyDer, pem::PemObject},
};
use std::env;
use std::{
    collections::HashMap,
    fmt::Debug,
    io::Error,
    net::IpAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    net::TcpListener,
    sync::Semaphore,
    time::{sleep, timeout},
};
use tokio_rustls::TlsAcceptor;

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
    limiter: ConnectionLimiter,
    tls_config: Arc<ServerConfig>,
    handler: Arc<H>,
}

/// Limits connections for a certain Tcp Connection.
#[derive(Clone, Debug)]
struct ConnectionLimiter {
    /// `HashMap` to store amount of connections per IP Address.
    inner: Arc<Mutex<HashMap<IpAddr, usize>>>,
    /// Desired limit on the amount of connections per IP Address.
    max_per_ip: usize,
}

/// RAII guard for each connection to be able to be dropped safely.
struct ConnectionGuard {
    limiter: ConnectionLimiter,
    addr: IpAddr,
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
        const MAX_CLIENTS: usize = 5000;
        let sem = Arc::new(Semaphore::new(MAX_CLIENTS));
        let acceptor = Arc::new(TlsAcceptor::from(Arc::clone(&self.tls_config)));
        loop {
            if self.closed.load(Ordering::SeqCst) {
                println!("We cannot take any new connections as the server was closed.");
                return;
            }
            match self.listener.accept().await {
                Ok((mut stream, addr)) => {
                    let ip = addr.ip();
                    if let Some(ip_guard) = self.limiter.try_connect(ip) {
                        let handler_clone = Arc::clone(&self.handler);
                        let sem_clone = Arc::clone(&sem);
                        let acceptor_clone = Arc::clone(&acceptor);
                        tokio::spawn(async move {
                            if let Ok(global_guard) = sem_clone.try_acquire() {
                                println!("Accepted a new connection");
                                let _guard = ip_guard; //move ownership
                                let _global_guard = global_guard; //move ownership

                                match TlsAcceptor::accept(&acceptor_clone, &mut stream).await {
                                    Ok(tls_stream) => {
                                        if let Err(e) = handle(tls_stream, &*handler_clone).await {
                                            eprintln!("Encountered error handling the stream: {e}");
                                        }
                                    }
                                    Err(err) => {
                                        eprintln!("Encountered error during TSL handshake: {err}");
                                    }
                                }
                            } else {
                                println!("Too many connections, rejecting client.");
                                let _ = stream.shutdown().await;
                            }
                        });
                    } else {
                        println!("Shutting down, request limit reached");
                        let _ = stream.shutdown().await;
                    }
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

impl ConnectionLimiter {
    fn new(max_per_ip: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_per_ip,
        }
    }

    fn try_connect(&self, addr: IpAddr) -> Option<ConnectionGuard> {
        let mut map = match self.inner.lock() {
            Ok(map) => map,
            Err(poisoned) => poisoned.into_inner(),
        };
        let count = map.entry(addr).or_insert(0);

        if *count >= self.max_per_ip {
            return None;
        }
        *count += 1;
        drop(map);
        Some(ConnectionGuard {
            limiter: self.clone(),
            addr,
        })
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let mut map = match self.limiter.inner.lock() {
            Ok(map) => map,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(count) = map.get_mut(&self.addr) {
            *count -= 1;
            if *count == 0 {
                map.remove(&self.addr);
            }
        }
    }
}

/// Helper function to extract a TLS server config.
///
/// This will for now only read the paths for the certificate and private key from the environment, but later be expanded to read configuration from a config file.
///
/// # Errors
///
/// Throws an Error if reading files fails.
fn build_tls_config() -> Result<ServerConfig, Error> {
    let cert_dir = env::var("TLS_CERT_PATH").unwrap_or_else(|_| "certs/cert.pem".to_string());
    let pk_dir = env::var("TLS_KEY_PATH").unwrap_or_else(|_| "certs/cert.key.pem".to_string());

    let config_builder = ServerConfig::builder().with_no_client_auth();
    let cert_chain: Vec<_> = CertificateDer::pem_file_iter(cert_dir)
        .map_err(Error::other)?
        .collect::<Result<_, _>>()
        .map_err(Error::other)?;
    let key_der = PrivatePkcs8KeyDer::from_pem_file(pk_dir)
        .map_err(Error::other)?
        .into();
    let config = config_builder
        .with_single_cert(cert_chain, key_der)
        .map_err(Error::other)?;

    Ok(config)
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
    let limiter = ConnectionLimiter::new(20);

    let mut config = build_tls_config()?;

    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let tls_config = Arc::new(config);
    let state = ServerState {
        listener,
        handler,
        limiter,
        tls_config,
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
async fn handle<H: Handler, S: AsyncRead + AsyncWrite + Unpin + Send>(
    mut stream: S,
    handler: &H,
) -> Result<(), HttpError> {
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
async fn process_request<H: Handler, S: AsyncRead + AsyncWrite + Unpin + Send>(
    mut stream: &mut S,
    handler: &H,
) -> Result<bool, HttpError> {
    const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(15);
    let request_future = request_from_reader(&mut stream);
    let request_res = timeout(KEEP_ALIVE_TIMEOUT, request_future).await;
    let request = match request_res {
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
    let keep_alive = Headers::get(&request.headers, "connection") != Some("close");

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
async fn write_response<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: &mut S,
    response: Response,
) -> Result<(), HttpError> {
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
    use rustls::{
        ClientConfig, ProtocolVersion, RootCertStore, ServerConfig,
        pki_types::{PrivatePkcs8KeyDer, ServerName},
    };
    use tokio::{
        io::AsyncWrite,
        time::{sleep, timeout},
    };
    use tokio_rustls::{TlsAcceptor, TlsConnector};

    use crate::{
        http::{
            request::{HttpError, Request},
            response::{Response, StatusCode, html_response},
        },
        runtime::{
            handler::Handler,
            server::{ConnectionLimiter, serve},
        },
    };

    use rcgen::{CertifiedKey, generate_simple_self_signed};

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
        let server = serve(1026, handler_arc)
            .await
            .expect("Failed to start server");

        let base_url = format!("https://127.0.0.1:{}", 1026);

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .danger_accept_invalid_certs(true)
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

        let base_url = format!("https://127.0.0.1:{}", 8081);

        let client = Client::builder()
            .danger_accept_invalid_certs(true)
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

        let base_url = format!("https://127.0.0.1:{}", 8082);

        let client = Client::builder()
            .danger_accept_invalid_certs(true)
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
        let server = serve(1025, handler_arc)
            .await
            .expect("Failed to start server");

        let base_url = format!("https://127.0.0.1:{}", 1025);

        let client = Client::builder()
            .danger_accept_invalid_certs(true)
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

    #[tokio::test]
    async fn rate_limit_enforcement() {
        let limiter = ConnectionLimiter::new(3);

        let guard1 = limiter.try_connect("192.0.2.1".parse().unwrap()).unwrap();
        let _guard2 = limiter.try_connect("192.0.2.1".parse().unwrap()).unwrap();
        let _guard3 = limiter.try_connect("192.0.2.1".parse().unwrap()).unwrap();

        assert!(limiter.try_connect("192.0.2.1".parse().unwrap()).is_none());

        assert!(limiter.try_connect("198.1.1.1".parse().unwrap()).is_some());

        drop(guard1);

        sleep(Duration::from_millis(100)).await;

        assert!(limiter.try_connect("192.0.2.1".parse().unwrap()).is_some());
    }

    #[tokio::test]
    async fn guard_auto_decrements_on_drop() {
        let limiter = ConnectionLimiter::new(3);

        {
            let _guard1 = limiter.try_connect("192.0.2.1".parse().unwrap()).unwrap();
            let _guard2 = limiter.try_connect("192.0.2.1".parse().unwrap()).unwrap();
            let _guard3 = limiter.try_connect("192.0.2.1".parse().unwrap()).unwrap();
        }

        sleep(Duration::from_millis(100)).await;

        assert!(limiter.try_connect("192.0.2.1".parse().unwrap()).is_some());
    }

    #[tokio::test]
    async fn server_can_establish_connection_via_tls() {
        let subject_names = vec!["localhost".to_string()];
        let CertifiedKey { cert, signing_key } =
            generate_simple_self_signed(subject_names).unwrap();

        let mut root_store = RootCertStore::empty();
        root_store.add(cert.der().clone()).unwrap();

        let client_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let der_bytes = signing_key.serialize_der();
        let private_key_der = PrivatePkcs8KeyDer::from(der_bytes);

        let server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert.der().clone()], private_key_der.into())
            .unwrap();

        let (client, server) = tokio::io::duplex(65536);
        let acceptor = TlsAcceptor::from(Arc::new(server_config));

        let connector = TlsConnector::from(Arc::new(client_config));
        let server_name = ServerName::try_from("localhost").unwrap();

        let (server_result, client_result) = tokio::join!(
            TlsAcceptor::accept(&acceptor, server),
            TlsConnector::connect(&connector, server_name, client),
        );

        let _server_stream = server_result.unwrap();
        let client_stream = client_result.unwrap();

        let result = client_stream.get_ref().1.protocol_version().unwrap();
        assert_eq!(result, ProtocolVersion::TLSv1_3);
    }
}
