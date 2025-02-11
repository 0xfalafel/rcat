use colored::Colorize;
use crate::Cli;

pub async fn client(host: &str, port: u16, cli: Cli) -> Result<(), String> {
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

pub async fn server(host: &str, port: u16, cli: Cli) -> Result<(), String> {
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