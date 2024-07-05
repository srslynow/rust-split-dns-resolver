use std::{error::Error, net::SocketAddr};

use clap::Parser;
use tokio::net::UdpSocket;
use tracing_subscriber::prelude::*;

pub mod dns_client;
pub mod dns_server;
pub mod parse_args;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // parse clap arguments
    let args = parse_args::Args::parse();

    // address to bind
    let addr = format!("{}:{}", args.host, args.port);

    // initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "split_dns_resolver=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let socket = UdpSocket::bind(&addr).await?;
    tracing::info!("Listening on: {}", socket.local_addr()?);

    const UPSTREAM_SERVERS: &[&str] = &[
        "8.8.8.8:53",
        "8.8.4.4:53",
        "1.1.1.1:53",
        "1.0.0.1:53",
        "192.168.50.2:53",
    ];

    let upsteam_dns_socket_addresses: Vec<SocketAddr> = UPSTREAM_SERVERS
        .iter()
        .map(|server| server.parse().unwrap())
        .collect();
    let mut dns_server =
        dns_server::DnsServer::new(upsteam_dns_socket_addresses, socket, vec![0; 1024]).await?;
    dns_server.run().await?;

    Ok(())
}
