use std::error::Error;
use std::net::SocketAddr;
use std::{env, io};
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use hickory_client::op::Message;
use hickory_client::udp::UdpClientStream;
use hickory_client::client::AsyncClient;
use hickory_proto::DnsHandle;

const UPSTREAM_SERVERS: &[&str] = &[
    "8.8.8.8:53",
    "8.8.4.4:53",
    "1.1.1.1:53",
    "1.0.0.1:53",
];

struct Server {
    socket: UdpSocket,
    buf: Vec<u8>,
    to_send: Option<(usize, SocketAddr)>,
}

impl Server {
    async fn query_upstream_servers(
        query_msg: Message,
    ) -> Result<Message, Box<dyn Error>> {
        let mut futures = FuturesUnordered::new();
        
        for &server in UPSTREAM_SERVERS {
            let query_msg_clone = query_msg.clone(); // Clone query_msg for each iteration
            let address = server.parse::<SocketAddr>()?;
            let stream = UdpClientStream::<UdpSocket>::new(address);
            let (client, bg) = AsyncClient::connect(stream).await?;
            tokio::spawn(bg);

            let client_future = async move {
                let response = timeout(Duration::from_secs(2), client.send(query_msg_clone).next()).await;
                match response {
                    Ok(Some(Ok(resp))) => Some(resp),
                    _ => None,
                }
            };

            futures.push(client_future);
        }

        while let Some(response) = futures.next().await {
            if let Some(valid_response) = response {
                return Ok(valid_response.into_message());
            }
        }

        Err("All upstream servers failed or returned invalid responses".into())
    }

    async fn run(self) -> Result<(), io::Error> {
        let Server {
            socket,
            mut buf,
            mut to_send,
        } = self;

        loop {
            if let Some((_size, peer)) = to_send {
                let query_msg = match Message::from_vec(&buf) {
                    Ok(msg) => msg,
                    Err(e) => {
                        eprintln!("Failed to parse query message: {}", e);
                        continue;
                    }
                };

                match Self::query_upstream_servers(query_msg.clone()).await {
                    Ok(mut response_message) => {
                        response_message.set_header(*response_message.header().clone().set_id(query_msg.header().id()));
                        if let Ok(response_vec) = response_message.to_vec() {
                            let amt = socket.send_to(&response_vec, &peer).await?;
                            println!("Sent {} bytes to {}, dns msg id: {}", amt, peer, query_msg.header().id());
                        } else {
                            eprintln!("Failed to serialize response message");
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to get valid response from upstream servers: {}", e);
                    }
                }
            }

            to_send = Some(socket.recv_from(&mut buf).await?);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:8080".to_string());

    let socket = UdpSocket::bind(&addr).await?;
    println!("Listening on: {}", socket.local_addr()?);

    let server = Server {
        socket,
        buf: vec![0; 1024],
        to_send: None,
    };

    server.run().await?;

    Ok(())
}
