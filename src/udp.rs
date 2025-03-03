use tokio::io::{stdin, stdout};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UdpSocket;
use tokio::select;

use colored::Colorize;
use rand::Rng;

use crate::Cli;

/// Return an UDP socket binding a random port above 49152
/// (ephemeral_port) and listening to 0.0.0.0
async fn bind_random_port() -> Result<UdpSocket, String> {

    // TODO: find on which interface we should listen. And bind only
    // that interface

    let mut rng = rand::rng();

    for _ in 0..10 {
        let port_number: u16 = rng.random_range(49152..u16::MAX);

        let res = UdpSocket::bind(format!("0.0.0.0:{}", port_number)).await;

        if let Ok(socket) = res {
            return Ok(socket)
        }
    }

    Err("Failed to bind port".to_string())
}

pub async fn udp_connect(host: &String, port: u16, cli: &Cli) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);

    let socket: UdpSocket = bind_random_port().await?;

    socket.connect(&addr)
        .await
        .map_err(|_| format!("failed to connect to {}", addr))?;

    if !cli.silent {
        eprintln!("Connected to {} (udp)", addr.green())
    }
    
    let mut stdin = stdin();
    let mut stdout = stdout();

    let mut stdin_buffer = [0; 512];
    let mut network_buffer  = [0; 512];

    let mut active = true;

    while active {
        select! {
            // Read from STDIN
            res = stdin.read(&mut stdin_buffer) => {
                match res {
                    Ok(n) if n !=0 => {
                      socket.send(&stdin_buffer[0..n])
                        .await
                        .map_err(|_| "failed to write to socket")?;
                    },
                    Ok(0) => {
                        active = false
                    },
                    Err(_) => {
                        return Err("failed to read from stdin".to_string())
                    },
                    Ok(1..) => unreachable!()
                }
            },
            // Read from Socket
            res = socket.recv(&mut network_buffer) => {
                match res {
                    Ok(n) if n !=0 => {
                      stdout.write(&network_buffer[0..n])
                        .await
                        .map_err(|_| "failed to write to stdout")?;
                    },
                    Ok(0) => {
                        active = false
                    },
                    Err(_) => {
                        return Err("failed to read from socket".to_string())
                    },
                    Ok(1..) => unreachable!()
                }
            }
        }
    }

    Ok(())
}

pub async fn udp_serve(host: &String, port: u16, cli: &Cli) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);

    let socket = UdpSocket::bind(&addr).await
        .map_err(|_| format!("failed to bind to {}", &addr))?;

    // Info message on successful bind
    if !cli.silent {
        eprintln!("Listening on {} (udp)", addr.cyan());
    }


    let mut stdin = stdin();
    let mut stdout = stdout();

    let mut stdin_buffer = [0; 512];
    let mut network_buffer  = [0; 512];

    let mut active = true;

    while active {
        select! {
            // Read from STDIN
            res = stdin.read(&mut stdin_buffer) => {
                match res {
                    Ok(n) if n !=0 => {
                      socket.send(&stdin_buffer[0..n])
                        .await
                        .map_err(|_| "failed to write to socket")?;
                    },
                    Ok(0) => {
                        active = false
                    },
                    Err(_) => {
                        return Err("failed to read from stdin".to_string())
                    },
                    Ok(1..) => unreachable!()
                }
            },
            // Read from Socket
            res = socket.recv(&mut network_buffer) => {
                match res {
                    Ok(n) if n !=0 => {
                      stdout.write(&network_buffer[0..n])
                        .await
                        .map_err(|_| "failed to write to stdout")?;
                    },
                    Ok(0) => {
                        active = false
                    },
                    Err(_) => {
                        return Err("failed to read from socket".to_string())
                    },
                    Ok(1..) => unreachable!()
                }
            }
        }
    }

    Ok(())
}