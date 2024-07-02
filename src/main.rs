use std::error::Error;

use clap::Parser;
use tokio::net::UdpSocket;
use tracing_subscriber::{prelude::*};

pub mod parse_args;
pub mod server;

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

    let server = server::Server {
        socket,
        buf: vec![0; 1024],
        to_send: None,
    };

    server.run().await?;

    Ok(())
}
