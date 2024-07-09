use core::time;
use std::net::SocketAddr;

use futures::StreamExt;
use hickory_client::client::AsyncClient;
use hickory_proto::udp::UdpClientStream;
use hickory_proto::{op::Message, DnsHandle};
use tokio::net::UdpSocket;

pub struct DnsClient {
    socket_addr: SocketAddr,
    client: AsyncClient,
}

impl DnsClient {
    /// Create a new DNS client with a single upstream server
    pub async fn new(socket_addr: SocketAddr) -> Option<Self> {
        // Create a new UDP socket and connect to the DNS server
        let stream =
            UdpClientStream::<UdpSocket>::with_timeout(socket_addr, time::Duration::from_secs(1));
        let client_connection_result = AsyncClient::connect(stream).await;
        // If the connection was successful, spawn a background task to handle the client
        match client_connection_result {
            Ok((client, bg)) => {
                tokio::spawn(bg);
                return Some(Self {
                    socket_addr,
                    client,
                });
            }
            Err(_error) => {
                tracing::error!("Failed DNS Client setup to socket addr: {}", socket_addr)
            }
        }
        None
    }

    /// Send a query to the DNS server
    pub async fn send_query(&self, query: Message) -> Option<Message> {
        // Send the query to the DNS server and await the response
        let response = self.client.send(query).next().await;
        // If the response was successful, return it
        if let Some(Ok(response)) = response {
            tracing::info!(
                "Received response from DNS server: {:?} and contains answer: {}",
                self.socket_addr,
                response.contains_answer()
            );
            if response.contains_answer() {
                return Some(response.into());
            }
        } else {
            tracing::error!("Failed to send query to DNS server: {}", self.socket_addr);
        }
        None
    }
}
