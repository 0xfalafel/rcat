use std::process::exit;

use tokio::io::{AsyncWriteExt, AsyncReadExt, ReadHalf, WriteHalf};
use tokio_util::sync::CancellationToken;

#[cfg(unix)]
use tokio::signal::unix::SignalKind;

use terminal_size::{Width, Height, terminal_size};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};

pub async fn upgrade_shell<T>(_reader: &mut ReadHalf<T>, writer: &mut WriteHalf<T>) -> Result<(), String> 
where 
    T: AsyncWriteExt + AsyncReadExt
{
    // launch /bin/bash with python
    match writer.write_all(b"python3 -c 'import pty;pty.spawn(\"/bin/bash\")'\n").await {
        Ok(_)  => {},
        Err(_) => return Err("Failed to initialize reverse shell.".to_string()),
    }

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Set Terminal size with `stty`
    let size = terminal_size();
    if let Some((Width(w), Height(h))) = size {
        // set the remote terminal size with stty
        let stty_command = format!("stty rows {} cols {}\n", h, w);
        
        match writer.write_all(stty_command.as_bytes()).await {
            Ok(_)  => {},
            Err(_) => return Err("Failed to write stty command to socket.".to_string()),
        }
    } else {
        return Err("Failed to obtain terminal size".to_string());
    }

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // set TERM env variable
    match writer.write_all(b"export TERM=xterm-256color\n").await {
        Ok(_)  => {},
        Err(_) => return Err("Failed to set XTERM variable.".to_string()),
    }
    
    // Set Terminal in raw mode
    match enable_raw_mode() {
        Ok(_) => {},
        Err(_) => return Err("Failed to enable raw mode".to_string()),
    }

    Ok(())
}

pub fn restore_terminal() {
    match disable_raw_mode() {
        Ok(_) => {},
        Err(_) => eprintln!("failed to restore terminal.\nUse the `reset` command to restore your terminal."),
    }
}

#[cfg(unix)]
pub async fn end_on_signal(cancel_token: CancellationToken) -> Result<(), String> {

    let mut sig_interrupt = tokio::signal::unix::signal(SignalKind::interrupt())
        .map_err(|_| "Failed to initialize interrupt signal handler")?;

    let mut sig_terminate = tokio::signal::unix::signal(SignalKind::terminate())
        .map_err(|_| "Failed to initialize terminate signal handler")?;

    tokio::select! { // if we received one of the signals
        _ = sig_interrupt.recv() => {},
        _ = sig_terminate.recv() => {},
    }

    cancel_token.cancel();
    restore_terminal();

    exit(0);
}

#[cfg(windows)]
pub async fn end_on_signal(cancel_token: CancellationToken) -> Result<(), String> {

    let mut sig_ctrlc = tokio::signal::windows::ctrl_c()
        .map_err(|_| "Failed to initialize interrupt signal handler")?;

    sig_ctrlc.recv().await;

    cancel_token.cancel();
    restore_terminal();

    exit(0);
}
