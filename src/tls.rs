use std::sync::Arc;
use tokio::{io::split, net::TcpStream};
use tokio_rustls::{rustls::{self, pki_types::ServerName, Error as RustlsError}, TlsConnector};
use colored::Colorize;

pub async fn connect_tls(host: &str, port: u16) -> Result<(), String> {
    let mut root_cert_store = rustls::RootCertStore::empty();

    // trust certificates accepted by Mozzila
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth(); // we don't do certificate authentication at the moment

    let tls_connector = TlsConnector::from(Arc::new(config));

    let addr = format!("{}:{}", host, port);
    let domain = ServerName::try_from(host)
        .map_err(|_| format!("{} is not a domain name", host))?
        .to_owned();


    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|_| format!("Could not connect to {}", addr))?;

    let stream = match tls_connector.connect(domain.clone(), stream).await {
        Ok(tls_stream) => tls_stream,

        Err(e) => {
            let error_detail= match e.downcast::<RustlsError>() {
                Ok(RustlsError::InvalidCertificate(_)) => "Invalid certificate.",
                _ => ""
            };

            return Err(format!("TLS error for {}: {}", addr, error_detail.red()));
        }
    };

    let (mut reader, mut writer) = split(stream);

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