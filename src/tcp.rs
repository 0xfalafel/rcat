use std::sync::Arc;
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
        let mut stdout = tokio::io::stdout();

        tokio::select! {
            _ = tokio::io::copy(&mut reader, &mut stdout) => {},
            _ = tokio::signal::ctrl_c() => {
                println!("Handling signal in Client_read");
                let _ = stdout.write(&[0x03]).await;
            }
        }
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

    let writer = Arc::new(tokio::sync::Mutex::new(writer));
    let writer2 = writer.clone();
    
    let client_read = tokio::spawn(async move {
        tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await
    });
    
    let client_write = tokio::spawn(async move {
        {
            let mut guard = writer.lock().await;  // Note: .await instead of .unwrap()
            tokio::io::copy(&mut tokio::io::stdin(), &mut *guard).await.unwrap();
        }
    });

    let ctrlc_handler: tokio::task::JoinHandle<Result<(), String>> = spawn(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                {
                    let mut soc = writer2.lock().await;
                    println!("Handling signal in client");
                    let _ = soc.write_all(&[0x03]);
                    let _ = soc.flush();
                }
                Ok(())
            }    
        }
    });


    tokio::select! {
        _ = client_read  => {},
        _ = client_write => {},
        _ = ctrlc_handler => {},
    }

    Ok(())
}

// /// Transmit Ctrl+C over the network
// async fn wait_for_signal_impl() {

//     tokio::select! {
//         _ = tokio::signal::ctrl_c() => {

//         }
//     };
// }