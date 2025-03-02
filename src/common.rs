use tokio::io::{ReadHalf, WriteHalf, AsyncReadExt, AsyncWriteExt};
use std::marker::Send;
use colored::Colorize;


use crate::{terminal_sheenanigans::upgrade_shell, Cli};

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
    }

    // Copy from network socket to stdout
    let client_read = tokio::spawn(async move {
        tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await
    });
    
    // Copy from stdin to network socket
    let client_write = tokio::spawn(async move {
        tokio::io::copy(&mut tokio::io::stdin(), &mut writer).await
    });

    tokio::select! {
        _ = client_read  => {},
        _ = client_write => {}
    }    

    Ok(())
}