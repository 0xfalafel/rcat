use tokio::io::{stdin, stdout};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UdpSocket;
use tokio::select;
use rand::Rng;


/// Return an UDP socket binding a random port above 49152
/// (ephemeral_port) and listening to 0.0.0.0
async fn bind_random_port() -> Result<UdpSocket, String> {

    // TODO: find on which interface we should listen. And bind only
    // that interface

    let mut rng = rand::rng();

    for _ in 0..10 {
        let port_number: u16 = rng.random_range(49152..u16::MAX);

        let res = UdpSocket::bind(format!("0.0.0.0:{}", port_number)).await;

        if let Ok(socket) = res {
            return Ok(socket)
        }
    }

    Err("Failed to bind port".to_string())
}

pub async fn udp_connect(host: &String, port: u16) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);

    let socket = bind_random_port().await?;

    let client = socket.connect(&addr)
        .await
        .map_err(|_| format!("failed to connect to {}", addr))?;

    // let (mut reader, mut writer) = client.into_split();

    Ok(())
}