use clap::{Parser, Subcommand};

mod connect;
mod server;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    commands: Commands
}

#[derive(Subcommand)]
enum Commands {
    /// Connect to server
    Connect {
        host: String,

        #[arg(short, long, value_parser = port_in_range)]
        port: u16
    },

    /// Start server
    Serve {
        #[arg(default_value = "127.0.0.1")]
        bind_host: String,

        #[arg(short, long, value_parser = port_in_range)]
        port: u16
    }
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


fn main() {
    let cli = Cli::parse();

    match &cli.commands {
        Commands::Connect { host, port } => {
            match connect::run(host, *port) {
                Ok(()) => {},
                Err(msg) => println!("failed: {}", msg)
            };
        },

        Commands::Serve { bind_host, port } => {
            println!("Listen to {}:{}", bind_host, port);
            match server::run(bind_host, *port) {
                Ok(()) => {},
                Err(msg) => println!("failed: {}", msg)
            }
        }
    }
}
