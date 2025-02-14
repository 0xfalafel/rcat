use std::sync::Arc;
use tokio::{net::tcp::OwnedWriteHalf, signal::unix::SignalKind, sync::Mutex};

use colored::Colorize;
use terminal_size::{Width, Height, terminal_size};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

    // Upgrade Reverse shell
    if cli.pwn {
        // let mut buf: [u8; 1024] = [0; 1024];

        // launch /bin/bash with python
        match writer.write_all(b"python3 -c 'import pty;pty.spawn(\"/bin/bash\")'\n").await {
            Ok(_)  => {},
            Err(_) => eprintln!("Failed to initialize reverse shell."),
        }

        // set TERM env variable
        match writer.write_all(b"export TERM=xterm-256color\n").await {
            Ok(_)  => println!("Set XTERM variable"),
            Err(_) => eprintln!("Failed to set XTERM variable."),
        }

        // Set Terminal size with `stty`
        let size = terminal_size();
        if let Some((Width(w), Height(h))) = size {
            println!("Terminal size: with={} , height={}", w, h);

            // set the remote terminal size with stty
            let stty_command = format!("stty rows {} cols {}\n", h, w);

            match writer.write_all(stty_command.as_bytes()).await {
                Ok(_)  => println!("Define terminal size with stty"),
                Err(_) => eprintln!("Failed to write stty command to socket."),
            }
        } else {
            eprintln!("Failed to obtain terminal size");
        }
    }


    let writer = Arc::new(Mutex::new(writer));

    // handle Ctrl-C
    tokio::spawn(handle_signal(SignalKind::interrupt(), 3, writer.clone()));

    // handle Ctrl-Z
    tokio::spawn(handle_signal(SignalKind::from_raw(20), 26, writer.clone()));

    
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
