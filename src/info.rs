pub mod io;
pub mod messages;

use crate::io::router::{RouterBuilder, RouterHandler};

use messages::requests::announce_shard_request::AnnounceShardRequest;
use messages::requests::get_client_shard_info_request::GetClientShardInfoRequest;
use messages::requests::get_shared_peers_request::GetSharedPeersRequest;
use messages::requests::query_version_request::QueryVersionRequest;
use messages::requests::read_request::ReadRequest;
use messages::requests::write_request::WriteRequest;

use messages::responses::announce_shard_response::AnnounceShardResponse;
use messages::responses::get_client_shard_info_response::GetClientShardInfoResponse;
use messages::responses::get_shared_peers_response::GetSharedPeersResponse;
use messages::responses::query_version_response::QueryVersionResponse;
use messages::responses::read_response::ReadResponse;
use messages::responses::write_response::WriteResponse;

use messages::requests::announce_shard_request::ShardType;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use anyhow::{Result};

struct ShardInfo {
    ip: u128,
    port: u16,
}

struct ReadWriteShardBlock {
    write_shard: ShardInfo,
    read_shards: Vec<ShardInfo>
}

struct InfoRouter {
    num_write_shard: u32,
    shard_blocks: Vec<ReadWriteShardBlock>
}

impl InfoRouter {
    pub fn new(num_write_shard: u32) -> Self {
        InfoRouter {
            shard_blocks: Vec::new(),
            num_write_shard: num_write_shard
        }
    }
}

impl RouterHandler for InfoRouter {
    /// This is when a read/write shard start up and announce their existance
    fn handle_announce_shard_request(&self, req: &AnnounceShardRequest) -> AnnounceShardResponse {
        match req.shard_type {
            ShardType::ReadShard => {
                req.
            }
            ShardType::WriteShard => {

            }
        }
        unimplemented!()
    }
    fn handle_announce_shard_response(&self, res: &AnnounceShardResponse) {
        unimplemented!()
    }

    /// This is when a read shard is trying to find its peers
    fn handle_get_shared_peers_request(&self, req: &GetSharedPeersRequest) -> GetSharedPeersResponse {
        unimplemented!()
    }
    fn handle_get_shared_peers_response(&self, res: &GetSharedPeersResponse) {
        unimplemented!()
    }

    /// This is when a client is trying to figure out who to connect to
    fn handle_get_client_shard_info_request(&self, req: &GetClientShardInfoRequest) -> GetClientShardInfoResponse {
        unimplemented!()
    }
    fn handle_get_client_shard_info_response(&self, res: &GetClientShardInfoResponse) {
        unimplemented!()
    }

    /// Unused requests
    fn handle_query_version_request(&self, req: &QueryVersionRequest) -> QueryVersionResponse {unimplemented!()}
    fn handle_read_request(&self, req: &ReadRequest) -> ReadResponse {unimplemented!()}
    fn handle_write_request(&self, req: &WriteRequest) -> WriteResponse {unimplemented!()}

    /// Unused responses
    fn handle_query_version_response(&self, res: &QueryVersionResponse) {unimplemented!()}
    fn handle_read_response(&self, res: &ReadResponse) {unimplemented!()}
    fn handle_write_response(&self, res: &WriteResponse) {unimplemented!()}

}


#[tokio::main] 
async fn main() -> Result<()> {
    let info_router = InfoRouter::new(4);
    let info_server = RouterBuilder::new(info_router, None);
    tokio::spawn(async move {
        info_server.listen().await?;
        Ok(())
    }).await?
}
