use clap::{Parser, Subcommand};
use tokio::io;

// mod connect;
mod server;
mod stream;
mod tls;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(default_value = "127.0.0.1")]
    host: String,

    #[arg(value_parser = port_in_range)]
    port: u16
}

fn port_in_range(s: &str) -> Result<u16, String> {
    let port: u16 = u16::from_str_radix(s, 10)
        .map_err(|_| format!("{} is not a valid port number", s))?;

    // port is a u16 value. Only 0 is an invalid port
    if port == 0 {
        Err(format!("{} is not a valid port number", s))
    } else {
        Ok(port)
    }
}

async fn run() -> Result<(), String> {
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();

    tokio::spawn(async move {
        tokio::io::copy(&mut stdin, &mut stdout).await.unwrap();
    }).await.unwrap();

    Ok(())
}


#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    stream::client(&cli.host, cli.port).await.unwrap();;
}
