use std::sync::Arc;
use tokio::{net::tcp::OwnedWriteHalf, signal::unix::SignalKind, sync::Mutex, io::split};

use colored::Colorize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::Cli;
use crate::terminal_sheenanigans::upgrade_shell;

pub async fn client(host: &str, port: u16, cli: &Cli) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);

    let client = tokio::net::TcpStream::connect(&addr)
        .await
        .map_err(|_| format!("Failed to connect to {}", addr.red()))?;

    if !cli.silent {
        eprintln!("Connected to {}", addr.green())
    }

    let (mut reader, mut writer) = split(client);

    let client_read = tokio::spawn(async move {
        tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await
    });
    
    let client_write = tokio::spawn(async move {
        tokio::io::copy(&mut tokio::io::stdin(), &mut writer).await
    });

    tokio::select! {
        _ = client_read  => {},
        _ = client_write => {}
    }
    
    Ok(())
}

pub async fn server(host: &str, port: u16, cli: &Cli) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|_| format!("failed to bind {}", addr))?;
    
    // Info message on successful bind
    if !cli.silent {
        eprintln!("Listening on {} (tcp)", addr.blue());
    }

    let (handle, remote_addr) = listener.accept().await.map_err(|_| "failed to accept connection")?;
    if !cli.silent {
        eprintln!("Connection received from {}", remote_addr.to_string().green());
    }

    let (mut reader, mut writer) = split(handle);

    // Upgrade Reverse shell
    if cli.pwn {
        match upgrade_shell(&mut reader, &mut writer).await {
            Ok(()) => {},
            Err(error_msg) => eprintln!("{}", error_msg.red())
        }
    }

    let writer = Arc::new(Mutex::new(writer));
    
    let client_write = tokio::spawn(async move {

        let mut buffer: [u8; 1024] = [0; 1024];
        let mut stdin = tokio::io::stdin();

        loop {
            match stdin.read(&mut buffer).await {
                Err(_) => {
                    return Err("Failed to read from stdin");
                },
                Ok(0) => break,
                Ok(ammount) => {
                    let mut guard = writer.lock().await;
                    guard.write(&buffer[..ammount]).await
                        .map_err(|_| "Failed to write to socket")?;
                }
            }
        }

        Ok(())
    });

    let client_read = tokio::spawn(async move {
        tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await
    });

    tokio::select! {
        _ = client_read  => {},
        _ = client_write => {},
        // _ = ctrlc_handler => {},
    }

    Ok(())
}

/// When a signal is received, transmit the corresponding ASCII control code
/// over the TCP connection.
/// Reference of ASCII control code: https://jvns.ca/ascii
#[allow(unused)]
async fn handle_signal(signum: SignalKind, ascii_control_code: u8, writer: Arc<Mutex<OwnedWriteHalf>>) -> Result<(), String> {
    let mut sig = tokio::signal::unix::signal(signum)
        .map_err(|_| "Failed to initialize signal")?;

    loop {
        sig.recv().await;
        let mut guard = writer.lock().await;
        guard.write(&[ascii_control_code]).await
            .map_err(|_| "Failed to transmit ASCII control code to socket.")?; // write 1 byte: the ascii code
    }
}
