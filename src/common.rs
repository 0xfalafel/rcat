use crossterm::terminal::enable_raw_mode;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use std::marker::Send;
use colored::Colorize;

use crate::{newline::NewlineReplacer, terminal_sheenanigans::upgrade_shell, Cli};

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

    let copy_from_stdin = match cli.crlf {
        false => tokio::io::copy(&mut tokio::io::stdin(), &mut writer).await,
        true =>  tokio::io::copy(&mut replacer, &mut writer).await,
    };

    let client_write = tokio::spawn(async move {
        copy_from_stdin
    });

    tokio::select! {
        _ = client_read  => {},
        _ = client_write => {}
    }    

    Ok(())
}