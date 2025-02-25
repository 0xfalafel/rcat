use std::{error::Error, sync::Arc};
use colored::Colorize;
use tokio::{io::split, net::TcpStream};

use tokio_rustls::{rustls::{self, client::danger::HandshakeSignatureValid, pki_types::{pem::PemObject, CertificateDer, ServerName}, RootCertStore, SignatureScheme}, TlsConnector};
use tokio_rustls::rustls::client::danger::{ServerCertVerified, ServerCertVerifier};
use crate::Cli;

pub async fn connect_tls(host: &str, port: u16, cli: &Cli) -> Result<(), String> {

    let root_cert_store = initialize_ca(&cli)
        .map_err(|_| "Failed to read Certificate Authority file")?;

    let mut config = rustls::ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth(); // we don't do certificate authentication at the moment

    // If --insecure is set. Ignore all TLS validation (Certificate, etc)
    if cli.insecure {
        config.dangerous().set_certificate_verifier(Arc::new(NoVerification));
    }

    let tls_connector = TlsConnector::from(Arc::new(config));

    let addr = format!("{}:{}", host, port);
    let domain = ServerName::try_from(host)
        .map_err(|_| format!("{} is not a domain name", host))?
        .to_owned();


    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|_| format!("Could not connect to {}", addr))?;

    let stream = tls_connector.connect(domain.clone(), stream)
        .await
        .map_err(|_| format!("Failed to etablish TLS connection with {} at address {}", domain.to_str(), addr))?;

    // Info message when connection is established
    if !cli.silent {
        eprintln!("Connected with TLS to {}", addr.green());
    }

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


/// Initilaize the Root Certificates that provides a root of trust
fn initialize_ca(cli: &Cli) -> Result<RootCertStore, Box<dyn Error + Send + Sync + 'static>> {
    let mut root_cert_store = rustls::RootCertStore::empty();

    if let Some(cafile) = &cli.cafile {
        for cert in CertificateDer::pem_file_iter(cafile)? {
            root_cert_store.add(cert?)?;
        }        
    } else {
        // trust certificates accepted by Mozzila
        root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());        
    }

    Ok(root_cert_store)
}


/*
    A Custom TLS verifier, to ignore TLS verification with the `-k` or `--insecure` option
*/

#[derive(Debug)]
struct NoVerification;

impl ServerCertVerifier for NoVerification {
    fn verify_server_cert(
            &self,
            _end_entity: &rustls::pki_types::CertificateDer<'_>,
            _intermediates: &[rustls::pki_types::CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: rustls::pki_types::UnixTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &rustls::pki_types::CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &rustls::pki_types::CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}
