use std::{future::Future, path::PathBuf, process::exit};
use colored::Colorize;

use clap::{error::Result, Parser};
use terminal_sheenanigans::{restore_terminal, end_on_signal};
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

mod tcp;
mod udp;
mod tls;
mod terminal_sheenanigans;
mod common;
mod newline;

#[derive(Parser,Default,Debug)]
//#[command(author, version, about, long_about = None)]
//#[command(propagate_version = true)]
struct Cli {
    /// Start a server listening for a connection.
    #[arg(short='l')]
    listen: bool, 

    /// Use TLS (to connect to a remote host, or with -l to start a TLS server).
    #[arg(short='t', long)]
    tls: bool,

    /// Use UDP (to connect to a remote host, or with -l to start a server).
    #[arg(short='u', long)]
    udp: bool,

    /// Replace '\n' with '\r\n'
    #[arg(short='c', long)]
    crlf: bool,

    /// Remove information messages.
    #[arg(short='s', long)]
    silent: bool,

    /// Ignore Certificate errors when connecting with --tls.
    #[arg(short='k', long)]
    insecure: bool,

    /// Automagicaly upgrade a Reverse Shell to a fully interactive Shell. 
    #[arg(long)]
    pwn: bool,

    #[arg(short='R')]
    no_autoresize: bool,

    /// Set the terminal to raw mode when we recieve a connection.
    #[arg(long)]
    raw: bool,

    /// Certificate autority to use to valide the remote host when connecting with TLS.
    #[arg(long)]
    cafile: Option<PathBuf>,
    
    // Options for the TLS server

    /// Certificate used by the TLS server
    #[arg(long)]
    cert: Option<PathBuf>,

    /// Private key used by the TLS server
    #[arg(long)]
    key: Option<PathBuf>,


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


fn async_run<F>(future: F, runtime: Runtime, token: CancellationToken) -> Result<(), String>
where 
    F: Future<Output = Result<(), String>>
{
    let res = runtime.block_on(async {
        tokio::select! {
            res = future => res,
            _ = token.cancelled() => Ok(()),
        }
    });

    // End the tokio runtime.
    // This will close the program when the connection is closed, or Ctrl+C is pressed.
    runtime.shutdown_background();

    return res
}


fn main() {
    let mut cli = Cli::parse();

    if cli.pwn { // Set the terminal to raw mode if we have --pwn
        cli.raw = true;
    }

    // If we have a TLS parameter, do a TLS connection
    if cli.cert.is_some() || cli.key.is_some() || cli.cafile.is_some() {
        cli.tls = true;
    }

    let (host, port) = match get_host_port(&cli) {
        Err(err_msg) => {
            eprintln!("{}", err_msg); exit(1)
        },
        Ok((host, port)) => (host, port)
    };

    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|_| {
        eprintln!("{}","Failed to initialize tokio runtime.".red()); exit(1)
    });

    let token = CancellationToken::new();

    // Reset the terminal if the process is killed with `interrupt` `terminate`
    runtime.spawn(end_on_signal(token.clone()));

    // We start a listener
    if cli.listen == true {
        let res = match cli {
            ref cli if cli.udp => async_run(udp::udp_serve(&host, port, &cli), runtime, token.clone()),
            ref cli if cli.tls => async_run(tls::server(&host, port, &cli), runtime, token.clone()),
            _  => async_run(tcp::server(&host, port, &cli), runtime, token.clone()),
        };

        if let Err(err_msg) = res {
            eprintln!("{}", err_msg)
        }

    // We connect to a remote server
    } else {
        let res = match cli {
            ref cli if cli.udp => async_run(udp::udp_connect(&host, port, &cli), runtime, token.clone()),
            ref cli if cli.tls => async_run(tls::connect_tls(&host, port, &cli), runtime, token.clone()),
            _ => async_run(tcp::client(&host, port, &cli), runtime, token.clone()),
        };

        if let Err(err_msg) = res {
            eprintln!("{}", err_msg)
        }
    }

    if cli.raw {
        restore_terminal();
    }
}
