use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub mod io;
pub mod messages;

use crate::io::router::{RouterBuilder, RouterHandler};
use messages::requests::{
    announce_shard_request::AnnounceShardRequest,
    get_client_shard_info_request::GetClientShardInfoRequest,
    get_shared_peers_request::GetSharedPeersRequest,
    query_version_request::QueryVersionRequest,
    read_request::ReadRequest,
    write_request::WriteRequest,
    get_version_request::GetVersionRequest,
};
use messages::responses::{
    announce_shard_response::AnnounceShardResponse,
    get_client_shard_info_response::GetClientShardInfoResponse,
    get_shared_peers_response::GetSharedPeersResponse,
    query_version_response::QueryVersionResponse,
    read_response::ReadResponse,
    write_response::WriteResponse,
    get_version_response::GetVersionResponse,
};

// Constants for shard types
const WRITE_SHARD: u8 = 0;
const READ_SHARD: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ShardInfo {
    ip: [u8; 16],
    port: u16,
}

#[derive(Debug)]
struct ShardState {
    write_shards: Vec<ShardInfo>,
    read_shards: Vec<ShardInfo>,
    // Map of write shard to its connected read shards
    write_shard_peers: HashMap<ShardInfo, Vec<ShardInfo>>,
}

struct InfoRouter {
    shard_state: Arc<Mutex<ShardState>>,
}

impl InfoRouter {
    fn new() -> Self {
        InfoRouter {
            shard_state: Arc::new(Mutex::new(ShardState {
                write_shards: Vec::new(),
                read_shards: Vec::new(),
                write_shard_peers: HashMap::new(),
            })),
        }
    }

    fn find_write_shard(&self, write_ip: &[u8; 16], write_port: u16) -> Option<ShardInfo> {
        let state = self.shard_state.lock().unwrap();
        state.write_shards.iter()
            .find(|shard| shard.ip == *write_ip && shard.port == write_port)
            .cloned()
    }
}

impl RouterHandler for InfoRouter {
    fn handle_announce_shard_request(&self, req: &AnnounceShardRequest) -> AnnounceShardResponse {
        let shard_info = ShardInfo {
            ip: req.ip,
            port: req.port,
        };

        // Lock the shard state and add the new shard
        let mut state = self.shard_state.lock().unwrap();
        
        match req.shard_type {
            WRITE_SHARD => {
                state.write_shards.push(shard_info.clone());
                println!("Registered new write shard at {}:{}", 
                    format!("{}.{}.{}.{}", req.ip[0], req.ip[1], req.ip[2], req.ip[3]), 
                    req.port
                );
                // Ensure an entry for this write shard in peer map
                state.write_shard_peers.entry(shard_info).or_default();
                return AnnounceShardResponse {
                    writer_number: 0, // TODO
                }
            },
            READ_SHARD => {
                state.read_shards.push(shard_info.clone());
                return AnnounceShardResponse {
                    writer_number: 0,
                }
            },
            _ => {
                // Invalid shard type
                return AnnounceShardResponse {
                    writer_number: 0,
                }
            }
        }
    }

    fn handle_get_client_shard_info_request(&self, _req: &GetClientShardInfoRequest) -> GetClientShardInfoResponse {
        let state = self.shard_state.lock().unwrap();
        
        GetClientShardInfoResponse {
            num_write_shards: state.write_shards.len() as u16,
            write_shard_info: state.write_shards.iter().cloned().map(|s| (s.ip, s.port)).collect(),
            read_shard_info: state.read_shards.iter().cloned().map(|s| (s.ip, s.port)).collect(),
        }
    }

    fn handle_get_shared_peers_request(&self, req: &GetSharedPeersRequest) -> GetSharedPeersResponse {
        let state = self.shard_state.lock().unwrap();
        
        // Calculate the index of the write shard based on the writer number
        let write_shard_index = req.writer_number as usize % state.write_shards.len();
        
        // Get the IP and port of the write shard
        let write_shard_info = state.write_shards[write_shard_index].clone();
        
        // Retrieve peers for this write shard
        let peers : Vec<([u8; 16], u16)> = state.write_shard_peers.get(&write_shard_info)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|peer| (peer.ip, peer.port))
            .collect();
        
        // Add the write shard's IP and port to the response
        let mut peer_ips = vec![(write_shard_info.ip, write_shard_info.port)];
        peer_ips.extend(peers);
        
        GetSharedPeersResponse { peer_ips }
    }

    // This method would typically be called when a read shard is registering its connection to a write shard
    fn handle_write_request(&self, req: &WriteRequest) -> WriteResponse {
        // If this method is used for registering peer connections, you'd implement the logic here
        // For now, it's unimplemented
        unimplemented!()
    }

    // Remaining methods can remain unimplemented for now
    fn handle_read_request(&self, _req: &ReadRequest) -> ReadResponse {
        unimplemented!()
    }

    // Other methods remain the same as in the previous implementation
    fn handle_get_client_shard_info_response(&self, _res: &GetClientShardInfoResponse) {
        unimplemented!()
    }

    // Placeholder or unimplemented methods
    fn handle_query_version_request(&self, _req: &QueryVersionRequest) -> QueryVersionResponse {
        unimplemented!()
    }

    // Placeholder response methods
    fn handle_announce_shard_response(&self, _res: &AnnounceShardResponse) {}
    fn handle_query_version_response(&self, _res: &QueryVersionResponse) {}
    fn handle_write_response(&self, _res: &WriteResponse) {}
    fn handle_read_response(&self, _res: &ReadResponse) {}
    fn handle_get_shared_peers_response(&self, _res: &GetSharedPeersResponse) {}
}

// Main function remains the same as in the previous implementation
#[tokio::main]
async fn main() -> Result<()> {
    // Create the router
    let info_router = InfoRouter::new();
    let router = RouterBuilder::new(info_router, None);

    // Bind the TCP listener
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Info server listening on 127.0.0.1:8080");

    loop {
        // Accept a new connection
        let (mut socket, _) = listener.accept().await?;
        
        // Clone the router for use in the async block
        let router_clone = router.clone();

        // Spawn a new task to handle the connection
        tokio::spawn(async move {
            let mut buffer = [0; 1024];

            // Read data from the socket
            let n = match socket.read(&mut buffer).await {
                Ok(n) if n == 0 => return, // Connection was closed
                Ok(n) => n,
                Err(_) => return,
            };

            // Process the received data using the router
            match router_clone.route(&buffer[..n]) {
                Ok(response) => {
                    // Send the response back to the client
                    if let Err(_) = socket.write_all(&response).await {
                        eprintln!("Failed to write response");
                    }
                }
                Err(e) => {
                    eprintln!("Routing error: {:?}", e);
                }
            }
        });
    }
}
