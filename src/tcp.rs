use std::sync::{Arc};//, Mutex};
use tokio::sync::Mutex;

use colored::Colorize;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, select, spawn};
use crate::Cli;

pub async fn client(host: &str, port: u16, cli: &Cli) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);

    let client = tokio::net::TcpStream::connect(&addr)
        .await
        .map_err(|_| format!("Failed to connect to {}", addr.red()))?;

    if !cli.silent {
        eprintln!("Connected to {}", addr.green())
    }

    let (mut reader, mut writer) = client.into_split();

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

    let (mut reader, mut writer) = handle.into_split();

    let shared_writer = Arc::new(Mutex::new(writer));
    let shared_writer2 = shared_writer.clone();
    
    let client_read = tokio::spawn(async move {
        tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await
    });
    
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
                    let mut guard = shared_writer.lock().await;
                    guard.write(&buffer[..ammount]).await
                        .map_err(|_| "Failed to write to socket")?;
                }
            }
        }

        Ok(())
    });

    let ctrlc_handler = tokio::spawn(async move {
        loop {
            tokio::signal::ctrl_c().await.unwrap();
            let mut guard = shared_writer2.lock().await;
            guard.write(b"\x03").await.unwrap();
        }
    });


    tokio::select! {
        _ = client_read  => {},
        _ = client_write => {},
        _ = ctrlc_handler => {},
    }

    Ok(())
}