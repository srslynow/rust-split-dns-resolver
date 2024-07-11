use std::error::Error;
use std::io;
use std::net::SocketAddr;

use hickory_client::op::Message;
use hickory_proto::op::Query;
use tokio::net::UdpSocket;
use tokio::time::Duration;
use ttlhashmap::TtlHashMap;

use crate::dns_client::{DnsClient, DnsResponseType};

pub struct DnsServer {
    pub socket: UdpSocket,
    pub buf: Vec<u8>,
    pub to_send: Option<(usize, SocketAddr)>,
    pub dns_clients: Vec<DnsClient>,
    pub ttl: u32,
}

impl DnsServer {
    pub async fn new(
        ttl: u32,
        upstream_dns_servers: Vec<SocketAddr>,
        socket: UdpSocket,
        buf: Vec<u8>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut dns_clients = Vec::new();
        for server in upstream_dns_servers {
            let dns_client = DnsClient::new(server)
                .await
                .expect(format!("Failed to create DNS client for server: {}", server).as_str());
            dns_clients.push(dns_client);
        }

        Ok(Self {
            ttl,
            socket,
            buf,
            to_send: None,
            dns_clients,
        })
    }

    pub async fn run(&mut self) -> Result<(), io::Error> {
        // hashmap which saves responses within ttl to cache
        let mut ttl_dns_hashmap: TtlHashMap<Query, Vec<u8>> =
            TtlHashMap::new(Duration::from_secs(self.ttl as u64));
        loop {
            if let Some((_size, peer)) = self.to_send {
                let query_msg = match Message::from_vec(&self.buf) {
                    Ok(msg) => msg,
                    Err(e) => {
                        tracing::error!("Failed to parse query message: {}", e);
                        continue;
                    }
                };

                tracing::info!("Query message: {}", query_msg.query().unwrap());

                // Check if the query is in the cache
                let query = query_msg.query().unwrap();
                let cache_result = ttl_dns_hashmap.get(&query);
                let results = match cache_result {
                    Some(cache_result) => {
                        tracing::info!("Found result in cache");
                        vec![Message::from_vec(cache_result)?]
                    }
                    None => {
                        tracing::info!("No result in cache, fetching fresh result");
                        // Query the upstream servers asynchronously
                        let futures = self.dns_clients.iter().map(|client| {
                            let query_msg_clone = query_msg.clone();
                            async move {
                                let response = client.send_query(query_msg_clone).await;
                                match response {
                                    Some(response) => Some(response),
                                    None => None,
                                }
                            }
                        });
                        // Wait for all the futures to complete
                        let results = futures::future::join_all(futures).await;
                        // Parse the responses into Vec<Message>
                        self.parse_dns_responses(results)
                    }
                };

                // Check if any of the responses are valid
                if results.len() > 0 {
                    let mut response_message = results[0].clone();
                    ttl_dns_hashmap.insert(query.clone(), response_message.to_vec()?);
                    // tracing::info!("Received response from upstream servers: {:?}", results);
                    response_message.set_header(
                        *response_message
                            .header()
                            .clone()
                            .set_id(query_msg.header().id()),
                    );
                    if let Ok(response_vec) = response_message.to_vec() {
                        // Send the response to the client
                        // TODO: Implement cache
                        let amt = self.socket.send_to(&response_vec, &peer).await?;
                        tracing::info!(
                            "Sent {} bytes to {}, dns msg id: {}",
                            amt,
                            peer,
                            query_msg.header().id()
                        );
                    } else {
                        tracing::error!("Failed to serialize response message");
                    }
                } else {
                    // failure to get valid response from upstream servers
                    // TODO: respond with no answer
                    tracing::error!("No usable response from upstream servers");
                }
            }
            self.to_send = Some(self.socket.recv_from(&mut self.buf).await?);
        }
    }

    /// Parse the DNS responses and return a Vec<Message>
    pub fn parse_dns_responses(
        &self,
        results: Vec<Option<(Message, DnsResponseType)>>,
    ) -> Vec<Message> {
        // Filter the results to Vec<(Message, DnsResponseType)>
        let dns_message_type_tuple = results
            .into_iter()
            .filter_map(|result| result)
            .collect::<Vec<(Message, DnsResponseType)>>();

        // Filter the results to only DNS messages with an address
        let dns_message_with_addr = dns_message_type_tuple
            .iter()
            .filter(|(_, response_type)| {
                if let DnsResponseType::ResponseWithAddr = response_type {
                    true
                } else {
                    false
                }
            })
            .map(|result| result.0.clone())
            .collect::<Vec<Message>>();
        // If there are any DNS messages with address, return them
        if dns_message_with_addr.len() > 0 {
            dns_message_with_addr
        } else {
            // If there are no responses with address, return all DNS Messages
            // (first DNS response without address is returned to the client)
            tracing::info!("No response with address found");
            let dns_message_without_addr = dns_message_type_tuple
                .iter()
                .map(|(message, _)| message.clone())
                .collect::<Vec<Message>>();
            dns_message_without_addr
        }
    }
}
