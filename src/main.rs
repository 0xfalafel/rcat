use std::process::exit;

use clap::Parser;
use tokio::io;

// mod connect;
mod server;
mod stream;
mod udp;
mod tls;

#[derive(Parser,Default,Debug)]
//#[command(author, version, about, long_about = None)]
//#[command(propagate_version = true)]
struct Cli {
    #[arg(short='l')]
    listen: bool,

    #[arg(short='t', long)]
    tls: bool,

    #[arg(short='u', long)]
    udp: bool,

    host: String,
    port: Option<String>
}

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

fn get_host_port(cli: &Cli) -> Result<(String, u16), String> {

    // We have a port passed as an argument
    if let Some(port) = &cli.port {
        
        let port = get_port(&port)?;
        return Ok((cli.host.clone(), port))

    // We don't have any `port` argument. Interpret the host argument as a port 
    // Host will be "0.0.0.0" by default when listening
    // and an error when connecting to a server
    } else {
        if cli.listen == false {
            return Err("An host and port parameters are requierd to connect to a server".to_string());
        }

        let port = get_port(&cli.host)?;
        return Ok(("0.0.0.0".to_string(), port));
    }
}


#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let (host, port) = match get_host_port(&cli) {
        Err(err_msg) => {
            println!("{}", err_msg); exit(1)
        },
        Ok((host, port)) => (host, port)
    };

    // We start a listener
    if cli.listen == true {
        let res = match cli {
            cli if cli.udp => udp::udp_serve(&host, port).await,
            _  => stream::server(&host, port).await,
        };

        if let Err(err_msg) = res {
            println!("{}", err_msg)
        }

    // We connect to a remote server
    } else {
        let res = match cli {
            cli if cli.udp => udp::udp_connect(&host, port).await,
            cli if cli.tls  => tls::connect_tls(&host, port).await,
            _ => stream::client(&host, port).await,
        };

        if let Err(err_msg) = res {
            println!("{}", err_msg)
        }
    }
}
