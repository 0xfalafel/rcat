use std::io::{Read, stdout, Write};
use std::net::TcpListener;

#[allow(unused)]
pub fn run(host: &str, port: u16) -> Result<(), String>{
    let addr = format!("{}:{}", host, port);
    println!("Connecting to {}", addr);

    let listener = TcpListener::bind(&addr)
        .map_err(|_| format!("failed to bind to {}", addr))?;

    
    for stream in listener.incoming() {
        
        match stream {
            Ok(mut s) => {
                println!("Connection accepted");

                let mut buf = [0; 128];
                let mut read_bytes = 0;

                while read_bytes == 0 {
                    read_bytes = s.read(&mut buf)
                        .map_err(|_| "failed to read from socket")?;

                    println!("recvied bytes {}", read_bytes);
                }

                stdout().write(&buf[0..read_bytes])
                    .map_err(|_| "failed to write to stdout")?;
                stdout().flush().unwrap();
            },

            Err(e) => {
                println!("Error while accepting incomming connection - {}", e);
            }
        }
    }

    Ok(())
}