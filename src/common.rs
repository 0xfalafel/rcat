use crossterm::terminal::enable_raw_mode;
use futures::lock::Mutex;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
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
    
    
    // We copy from stdin to the network socket.        
    // If cli.crlf flag is present: we copy from the NewlineReplacer.

    let mut replacer = NewlineReplacer::new(tokio::io::stdin());
    let writer = Arc::new(Mutex::new(writer));

    let copy_from_stdin = match cli.crlf {
        false => copy(&mut tokio::io::stdin(), writer.clone()).await,
        true =>  copy(&mut replacer, writer.clone()).await,
    };

    let client_write = tokio::spawn(async move {
        copy_from_stdin
    });

    if cli.raw {
        tokio::spawn(autoresize_terminal(writer));
    }

    tokio::select! {
        _ = client_read  => {},
        _ = client_write => {}
    }    

    Ok(())
}

pub async fn copy<R, T>(mut reader: R, writer: Arc<Mutex<WriteHalf<T>>>) -> Result<(), String> 
where
    R: AsyncRead + Unpin,
    T: AsyncWriteExt,
{

    let mut buffer = [0; 1024];

    loop {
        // Read data from the reader
        let n = match reader.read(&mut buffer).await {
            Ok(0) => break, // End of stream
            Ok(n) => n,
            Err(e) => return Err(format!("Failed to read from socket: {}", e)),
        };

        // Write data to the writer
        let mut writer = writer.lock().await;
        if let Err(e) = writer.write_all(&buffer[..n]).await {
            return Err(format!("Failed to write to socket: {}", e));
        }
    }

    Ok(())
}