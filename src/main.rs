use std::{future::Future, process::exit};
use colored::Colorize;

use clap::{error::Result, Parser};
use terminal_sheenanigans::{restore_terminal, end_on_signal};
use tokio::{runtime::Runtime, signal::unix::SignalKind};
use tokio_util::sync::CancellationToken;

// mod connect;
mod server;
mod tcp;
mod udp;
mod tls;
mod terminal_sheenanigans;

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

    #[arg(short='s', long)]
    silent: bool,

    #[arg(short='k', long)]
    insecure: bool,

    #[arg(short='S', long)]
    ignore_signals: bool,

    #[arg(long)]
    pwn: bool,

    // #[arg(short='r', long)]
    // raw: bool,

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


fn async_run<F>(future: F, cli: &Cli, runtime: Runtime, token: CancellationToken) -> Result<(), String>
where 
    F: Future<Output = Result<(), String>>
{
    let res = runtime.block_on(async {
        tokio::select! {
            res = future => res,
            _ = token.cancelled() => Ok(()),
            _ = tokio::signal::ctrl_c(), if !cli.ignore_signals => {println!("End process"); Ok(())} // if -S, don't close on Ctrl-C
        }
    });

    // End the tokio runtime.
    // This will close the program when the connection is closed, or Ctrl+C is pressed.
    runtime.shutdown_background();

    return res
}


fn main() {
    let cli = Cli::parse();

    // if cli.ignore_signals {
    //     // Setup a handler for Ctrl-C that will do nothing
    //     // when the signal is received
    //     if let Err(_) = ctrlc::set_handler(move || {println!("Handling signal in initial handler");}) {
    //         eprintln!("Error setting Ctrl-C handler");
    //     }
    // }

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
    if cli.pwn {
        runtime.spawn(end_on_signal(SignalKind::interrupt(), token.clone()));
        runtime.spawn(end_on_signal(SignalKind::terminate(), token.clone()));
    }

    // We start a listener
    if cli.listen == true {
        let res = match cli {
            ref cli if cli.udp => async_run(udp::udp_serve(&host, port), &cli, runtime, token.clone()),
            _  => async_run(tcp::server(&host, port, &cli), &cli, runtime, token.clone()),
        };

        if let Err(err_msg) = res {
            eprintln!("{}", err_msg)
        }

    // We connect to a remote server
    } else {
        let res = match cli {
            ref cli if cli.udp => async_run(udp::udp_connect(&host, port), &cli, runtime, token.clone()),
            ref cli if cli.tls => async_run(tls::connect_tls(&host, port, &cli), &cli, runtime, token.clone()),
            _ => async_run(tcp::client(&host, port, &cli), &cli, runtime, token.clone()),
        };

        if let Err(err_msg) = res {
            eprintln!("{}", err_msg)
        }
    }

    if cli.pwn {
        restore_terminal();
    }
}
