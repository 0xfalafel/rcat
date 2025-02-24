use std::process::exit;

use tokio::net::tcp::{OwnedWriteHalf, OwnedReadHalf};
use tokio::io::AsyncWriteExt;
use tokio::signal::unix::SignalKind;
use tokio_util::sync::CancellationToken;

use terminal_size::{Width, Height, terminal_size};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};

pub async fn upgrade_shell(_reader: &mut OwnedReadHalf, writer: &mut OwnedWriteHalf) -> Result<(), String> {
    // launch /bin/bash with python
    match writer.write_all(b"python3 -c 'import pty;pty.spawn(\"/bin/bash\")'\n").await {
        Ok(_)  => {},
        Err(_) => return Err("Failed to initialize reverse shell.".to_string()),
    }

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Set Terminal size with `stty`
    let size = terminal_size();
    if let Some((Width(w), Height(h))) = size {
        println!("Terminal size: with={} , height={}", w, h);
        
        // set the remote terminal size with stty
        let stty_command = format!("stty rows {} cols {}\n", h, w);
        println!("stty command: {}", stty_command);
        
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
        Err(_) => eprintln!("failed to restore terminal"),
    }
}

pub async fn end_on_signal(signum: SignalKind, cancel_token: CancellationToken) -> Result<(), String> {
    let mut sig = tokio::signal::unix::signal(signum)
        .map_err(|_| "Failed to initialize signal")?;

    sig.recv().await;
    restore_terminal();

    cancel_token.cancel();

    exit(0);
}