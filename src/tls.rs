use std::{error::Error, path::PathBuf, sync::Arc};
use colored::Colorize;
use tokio::{io::split, net::TcpStream};

use tokio_rustls::{rustls::{self, client::danger::HandshakeSignatureValid, pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer, ServerName}, RootCertStore, ServerConfig, SignatureScheme}, TlsConnector};
use tokio_rustls::rustls::client::danger::{ServerCertVerified, ServerCertVerifier};
use rcgen::{generate_simple_self_signed, CertifiedKey};
use crate::{common::read_write, Cli};

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

    let (reader, writer) = split(stream);

    read_write(reader, writer, cli).await
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


/// Handle connections with a TLS server
pub async fn server(host: &str, port: u16, cli: &Cli) -> Result<(), String>{

    let addr = format!("{}:{}", host, port);

    if cli.cert.is_none() && !cli.self_signed {
        return Err(String::from("A certificate (--cert) is required to create a TLS handler."))
    }
    
    if cli.key.is_none() && !cli.self_signed {
        return Err(String::from("A private key (--key) is required to create a TLS handler."))
    }

    // Generate a self-signed certificate for our server.
    if cli.self_signed {
        let subject_alt_names = vec![addr];
        let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names).unwrap();

        println!("{}", cert.pem());
        println!("{}", key_pair.serialize_pem());

        std::process::exit(0);
    }

    let tls_config = build_tls_server_config(
        cli.cert.as_ref().unwrap(),
        cli.key.as_ref().unwrap()
    )?;

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|_| format!("failed to bind {}", addr))?;

    // Info message on successful bind
    if !cli.silent {
        eprintln!("Listening on {} (tcp/tls)", addr.cyan());
    }

    // Accept the connection
    let (tcp_stream, remote_addr) = listener.accept().await
        .map_err(|_| "failed to accept connection")?;

    if !cli.silent {
        eprintln!("Connection received from {}", remote_addr.to_string().green());
    }

    let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls_config));
    let stream = tls_acceptor.accept(tcp_stream)
        .await
        .map_err(|_| "Failed to establish TLS connection.")?;

    let (reader, writer) = split(stream);
    
    read_write(reader, writer, cli).await
}

/// generate an RSA private key
fn generate_key() {
    
}

fn build_tls_server_config(cert_path: &PathBuf, private_key_path: &PathBuf) -> Result<ServerConfig, String> {

    // Read the private key
    let private_key = match PrivateKeyDer::from_pem_file(private_key_path) {
        Ok(private_key) => private_key,
        Err(_) => return Err("Failed to parse Private Key file".to_string())
    };

    // Read the certificate file
    let certs = match CertificateDer::pem_file_iter(cert_path) {
        Ok(certs) => certs,
        Err(_) => return Err("Failed to parse Certificate file.".to_string())
    };

    let certs = certs
        .map(|cert| {
            cert.map_err(|_| "Failed to parse certificate file".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;


    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, private_key)
        .map_err(|_| "Failed to initialize TLS Server configuration")?;

    Ok(config)
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
