use std::{process::exit, time::Duration};
use colored::Colorize;
use tokio::sync::Mutex;
use std::sync::Arc;

use tokio::io::{AsyncWriteExt, AsyncReadExt, ReadHalf, WriteHalf};
use tokio_util::sync::CancellationToken;

#[cfg(unix)]
use tokio::signal::unix::SignalKind;
use tokio::time::{sleep, timeout};

use terminal_size::{Width, Height, terminal_size};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};


pub async fn upgrade_shell<T>(reader: &mut ReadHalf<T>, writer: &mut WriteHalf<T>) -> Result<(), String> 
where 
    T: AsyncWriteExt + AsyncReadExt
{
    
    let mut buf = vec![0;1024];
    
    // Read and ignore the inital input that can be generated
    // by the reverse shell
    let time_limit = Duration::from_millis(500);
    let _res = timeout(time_limit, async {
        loop {
            let _ = match reader.read(&mut buf).await {
                Ok(size) => size,
                Err(_) => 0 //return Err("Initial read failed.".to_string()),
            };
        }
    }).await;

    // launch /bin/bash with python
    match writer.write_all(b"uname -s\n").await {
        Ok(_)  => {},
        Err(_) => return Err("Failed to detect the remote operating system.".to_string()),
    }

    
    let mut size = match reader.read(&mut buf).await {
        Ok(size) => size,
        Err(_) => return Err("Failed to detect the remote operating system.".to_string()),
    };

    // If we just have an anwser like `$ `, read again
    if size < 4 {
        size = match reader.read(&mut buf).await {
            Ok(size) => size,
            Err(_) => return Err("Failed to detect the remote operating system.".to_string()),
        };    
    }

    let uname = String::from_utf8_lossy(&buf[..size]);
    if uname.contains("Linux") {
        upgrade_shell_linux(reader, writer).await
        
    } else {
        eprint!("{}", "[*] Only Linux and Mac OS are supported at the moment for the shell upgrade".yellow());
        Ok(())
    }
}

/// Detect if the size of the terminal windows has changed
/// and resize the remote terminal if this happens
pub async fn autoresize_terminal<T>(writer: Arc<Mutex<WriteHalf<T>>>) -> Result<(), String>
where T: AsyncWriteExt + Send + 'static,
{
    let (mut intial_width, mut initial_height) = match terminal_size() {
        Some((Width(width), Height(height))) => (width, height),
        None => return Err("Failed to obtain terminal size".to_string())
    };

    loop {
        sleep(Duration::from_millis(400)).await;

        if let Some((Width(width), Height(height))) = terminal_size() {
            if width != intial_width || height != initial_height {
                let mut writer = writer.lock().await;
                
                // Send Ctrl-Z signal to the remote terminal
                if let Err(e) = writer.write_all(&[0x1a]).await {
                    return Err(format!("Failed to write to socket: {}", e));
                }

                // This is 100 ms delay is needed for some apps like `vim` that take time to react to the Ctrl-Z signal
                sleep(Duration::from_millis(100)).await;

                // set the remote terminal size with stty
                let stty_command = format!("stty rows {} cols {}; fg 2>/dev/null\n", height, width);
                if let Err(e) = writer.write_all(stty_command.as_bytes()).await {
                    return Err(format!("Failed to write to socket: {}", e));
                }
                    
                intial_width = width;
                initial_height = height;
            }
        }
    }
} 

pub async fn upgrade_shell_linux<T>(_reader: &mut ReadHalf<T>, writer: &mut WriteHalf<T>) -> Result<(), String> 
where 
    T: AsyncWriteExt + AsyncReadExt
{
    // Set Terminal in raw mode
    match enable_raw_mode() {
        Ok(_) => {},
        Err(_) => return Err("Failed to enable raw mode".to_string()),
    }

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
