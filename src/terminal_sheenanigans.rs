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

use crate::upgrade_windows_shell::WINDOWS_UPGRADE;

enum TerminalUpgradeError {
    FailedToReadAnwser,
    // FailedUtf8Decoding,
}

#[derive(Debug, PartialEq)]
pub enum OS {
    Unix,
    Windows,
    Unknown
}

pub async fn upgrade_shell<T>(reader: &mut ReadHalf<T>, writer: &mut WriteHalf<T>) -> Result<OS, String> 
where 
    T: AsyncWriteExt + AsyncReadExt
{
    let mut buf = vec![0;4096];
    
    // Read and ignore the inital input that can be generated
    // by the reverse shell
    let time_limit = Duration::from_millis(500);
    let _res = timeout(time_limit, async {
        loop {
            let _size = match reader.read(&mut buf).await {
                Ok(size) => size,
                Err(_) => 0 //return Err("Initial read failed.".to_string()),
            };
        }
    }).await;

    let os = detect_os(reader, writer).await?;
    
    match os {
        OS::Unix => upgrade_shell_linux(reader, writer).await?,
        OS::Windows => upgrade_shell_windows(reader, writer).await?,
        _ => eprint!("{}", "[*] Only Linux and Windows are supported at the moment for the shell upgrade".yellow())
    }

    Ok(os)
}

async fn send_command_read_anwser<T>(command: &str, reader: &mut ReadHalf<T>, writer: &mut WriteHalf<T>) -> Result<String, TerminalUpgradeError>
where
    T: AsyncWriteExt + AsyncReadExt
{
    let mut buf = vec![0;4096];

    // Test if we have a Unix system
    match writer.write_all(format!("{command}\n").as_bytes()).await {
        Ok(_)  => {},
        Err(_) => return Err(TerminalUpgradeError::FailedToReadAnwser),
    };

    let mut size = match reader.read(&mut buf).await {
        Ok(size) => Ok(size),
        Err(_) => Err(TerminalUpgradeError::FailedToReadAnwser),
    }?;

    // If we just have an anwser like `$ `, read again
    if size < 4 {
        size = match reader.read(&mut buf).await {
            Ok(size) => Ok(size),
            Err(_) => Err(TerminalUpgradeError::FailedToReadAnwser),
        }?;
    }

    let mut res = String::from_utf8_lossy(&buf[..size]);

    // If the command is reflected, read again
    if res.contains(command) {
        size = match reader.read(&mut buf).await {
            Ok(size) => size,
            Err(_) => return Err(TerminalUpgradeError::FailedToReadAnwser),
        };

        res = String::from_utf8_lossy(&buf[..size]);
    }

    Ok(res.to_string())
}

pub async fn detect_os<T>(reader: &mut ReadHalf<T>, writer: &mut WriteHalf<T>) -> Result<OS, String> 
where
    T: AsyncWriteExt + AsyncReadExt
{
    // Test if we have a Unix system
    let res_command= match send_command_read_anwser("uname -s", reader, writer).await {
        Ok(res_command)  => res_command,
        Err(_) => return Err("Failed to detect the remote operating system.".to_string()),
    };

    if res_command.contains("Linux") {
        return Ok(OS::Unix)
    }

    println!("final res: {}", res_command);

    // Let's test if it's a Windows
    let res_command= match send_command_read_anwser("systeminfo /?", reader, writer).await {
        Ok(res_command)  => res_command,
        Err(_) => return Err("Failed to detect the remote operating system.".to_string()),
    };

    if res_command.contains("SYSTEMINFO") {
        return Ok(OS::Windows)
    }
    
    Ok(OS::Unknown)
}


/// Detect if the size of the terminal windows has changed
/// and resize the remote terminal if this happens
pub async fn autoresize_terminal<T>(writer: Arc<Mutex<WriteHalf<T>>>, os: OS) -> Result<(), String>
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

                if os == OS::Unix {

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
                } else if os == OS::Windows {
                    let resize_terminal = format!("\
                        $Host.UI.RawUI.BufferSize = New-Object Management.Automation.Host.Size (1000, 9999);\
                        $Host.UI.RawUI.WindowSize = New-Object System.Management.Automation.Host.Size ({}, {});",
                        width, height
                    );

                    if let Err(e) = writer.write_all(resize_terminal.as_bytes()).await {
                        return Err(format!("Failed to write to socket: {}", e));
                    }
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

pub async fn upgrade_shell_windows<T>(_reader: &mut ReadHalf<T>, writer: &mut WriteHalf<T>) -> Result<(), String> 
where 
    T: AsyncWriteExt + AsyncReadExt
{
    eprintln!("Detected {}. {} the {}.", "Windows".blue(), "Upgrading".yellow(), "shell".yellow());

    // Copy the Windows shell upgrade script
    for line in WINDOWS_UPGRADE.lines() {
        match writer.write_all(format!("{}\n", line).as_bytes()).await {
            Ok(_)  => {},
            Err(_) => return Err("Failed to copy Windows shell upgrade script.".to_string()),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Obtain terminal size
    let (w, h) = match terminal_size() {
        Some((Width(w), Height(h))) => (w, h),
        None => (80, 20)
    };

    let upgrade_shell_command = format!(
        "$output = [UpgradeMainClass]::UpgradeMain(@({}, {}))\n",
        h, w
    );

    match writer.write_all(upgrade_shell_command.as_bytes()).await {
        Ok(_)  => {},
        Err(_) => return Err("Failed to write UpgradeMainClass command.".to_string()),
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
