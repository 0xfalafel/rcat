pub async fn client() -> Result<(), String> {
    let client = tokio::net::TcpStream::connect("localhost:2323")
        .await
        .map_err(|_| "failed to connect")?;

    let (mut reader, mut writer) = client.into_split();

    let client_read = tokio::spawn(async move {
        tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await
    });
    
    let client_write = tokio::spawn(async move {
        tokio::io::copy(&mut tokio::io::stdin(), &mut writer).await
    });

    tokio::select! {
        _ = client_read  => {},
        _ = client_write => {}
    }

    Ok(())
}