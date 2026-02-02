use std::{io::Error, net::UdpSocket};

/**
 * UDP server.
 */
fn main() -> Result<(), Error> {
    let socket = UdpSocket::bind("127.0.0.1:8080")?;
    let mut buf = [0u8; 1024];
    loop {
        println!(">");
        let (len, address) = socket.recv_from(&mut buf)?;
        let text = String::from_utf8_lossy(&buf[..len]);
        println!("read: {}", text);
        socket.send_to(&buf, address).unwrap();
    }
}