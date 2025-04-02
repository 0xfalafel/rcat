use crossterm::terminal::enable_raw_mode;
use tokio::sync::Mutex;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use std::pin::Pin;
use std::{marker::Send, sync::Arc};
use colored::Colorize;

use crate::{newline::NewlineReplacer, terminal_sheenanigans::{autoresize_terminal, upgrade_shell}, Cli};

/// Copy from stdin to network, and from network to stdout.
/// It will also run upgrade_shell() if specified
pub async fn read_write<T>(mut reader: ReadHalf<T>, mut writer: WriteHalf<T>, cli: &Cli) -> Result<(), String> 
where 
    T: AsyncWriteExt + AsyncReadExt + Send + 'static
{
    // Upgrade Reverse shell
    if cli.pwn {
        match upgrade_shell(&mut reader, &mut writer).await {
            Ok(()) => {},
            Err(error_msg) => eprintln!("{}", error_msg.red())
        }
    // If we don't have --pwn, but still have --raw option activated
    } else if cli.raw {
        match enable_raw_mode() {
            Ok(_) => {},
            Err(_) => return Err("Failed to enable raw mode".to_string()),
        }
    }

    // Copy from network socket to stdout
    let client_read = tokio::spawn(async move {
        tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await
    });
    
    let arc_writer = Arc::new(Mutex::new(writer));

    let crlf: bool = cli.crlf;
    let socker_writer = arc_writer.clone();

    let client_write = tokio::spawn(async move {
        copy_from_stdin(socker_writer, crlf).await
    });

    if cli.pwn {
        tokio::spawn(autoresize_terminal(arc_writer.clone()));
    }

    tokio::select! {
        _ = client_read  => {},
        _ = client_write => {}
    }    

    Ok(())
}

pub async fn copy_from_stdin<T>(writer_mutex: Arc<Mutex<WriteHalf<T>>>, crlf: bool) -> Result<(), String> 
where
    T: AsyncWriteExt + 'static,
{
    let mut stdin: Pin<Box<dyn AsyncRead + Send>> = match crlf {
        true => Box::pin(NewlineReplacer::new(tokio::io::stdin())),
        false => Box::pin(tokio::io::stdin()),
    };

    let mut buffer = [0; 1024];

    loop {
        // Read data from the reader
        let n = match stdin.read(&mut buffer).await {
            Ok(0) => break, // End of stream
            Ok(n) => n,
            Err(e) => return Err(format!("Failed to read from socket: {}", e)),
        };

        // Write data to the writer
        let mut writer = writer_mutex.lock().await;
        if let Err(e) = writer.write_all(&buffer[..n]).await {
            return Err(format!("Failed to write to socket: {}", e));
        }
    }

    Ok(())
}