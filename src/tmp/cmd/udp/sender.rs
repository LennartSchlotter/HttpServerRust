use std::io::{self, Error};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

/**
 * UDP sender.
 */
fn main() -> Result<(), Error> {
    // Determine both the address we are sending from and the address we are sending to
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    // bind the address to a UdpSocket
    let socket_res = UdpSocket::bind(address);
    let socket = match socket_res {
        Ok(socket) => socket,
        Err(error) => panic!("Problem with binding the socket: {error:?}"),
    };

    // This does not establish a real connection (UDP is connectionless) but instead associates an address with the socket
    socket.connect(server_address)?;
    loop {
        println!(">");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let line = input.trim_end_matches(&['\r', '\n'][..]);

        // After connecting the socket to a specific address, we can use send() instead of send_to() to default to the connected address.
        socket.send(line.as_bytes())?;
    }
}