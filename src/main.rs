use std::process::exit;

use clap::Parser;
use tokio::io;

// mod connect;
mod server;
mod stream;
mod tls;

#[allow(unused)]

#[derive(Parser,Default,Debug)]
//#[command(author, version, about, long_about = None)]
//#[command(propagate_version = true)]
struct Cli {
    #[arg(short = 'l', required = false)]
    listen: bool,


    // #[arg(default_value = "127.0.0.1")]
    #[arg(required_unless_present = "listen", default_value = "127.0.0.1")]
    host: String,

    // #[arg(value_parser = get_port)]
    port: Option<String>
}

#[allow(unused)]
fn get_port(s: &str) -> Result<u16, String> {
    let port: u16 = u16::from_str_radix(s, 10)
        .map_err(|_| format!("{} is not a valid port number", s))?;

    // port is a u16 value. Only 0 is an invalid port
    if port == 0 {
        Err(format!("{} is not a valid port number", s))
    } else {
        Ok(port)
    }
}

#[allow(unused)]
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

    // We start a listener
    if cli.listen == true {

        // We have a port passed as an argument
        if let Some(port) = cli.port {
            
            let port = match get_port(&port) {
                Err(err_msg) => {println!("{}", err_msg); exit(1)},
                Ok(port) => port
            };

            if let Err(err_msg) = stream::server(&cli.host, port).await {
                println!("{}", err_msg)
            }

        // We don't have any `port` argument. Interpret the host argument as a port 
        } else {
            let port = match get_port(&cli.host) {
                Err(err_msg) => {
                    println!("{}", err_msg);
                    exit(1)
                },
                Ok(port) => port
            };

            match stream::server("0.0.0.0", port).await {
                Ok(_) => {},
                Err(err_msg) => println!("{}", err_msg)
            }

            if let Err(err_msg) = stream::server("0.0.0.0", port).await {
                println!("{}", err_msg)
            }
        }
        
    // We connect to a remote server
    } else {

        // We have a port passed as an argument
        if let Some(port) = cli.port {

            let port = match get_port(&port) {
                Err(err_msg) => { println!("{}", err_msg); exit(1) },
                Ok(port) => port
            };
    
            if let Err(err_msg) = stream::client(&cli.host, port).await {
                println!("{}", err_msg)
            }
        } else {
            println!("A port argument is required.")
        }
    }

}
