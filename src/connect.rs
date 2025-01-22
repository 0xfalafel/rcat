use std::io::Write;
use std::net::TcpStream;

pub fn run(host: &str, port: u16) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);
    println!("Connect to {}", addr);

    let mut client = TcpStream::connect(&addr)
        .map_err(|_| format!("failed to connect to {}", addr))?;

    client.write("Hello TCP".as_bytes())
        .map_err(|_| format!("failed to send"))?;

    Ok(())
}