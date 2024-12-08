use crate::io::write::write_message;
use crate::messages::message::{AsAny, Message};
use crate::messages::requests::get_shared_peers_request::GetSharedPeersRequest;
use crate::messages::responses::get_shared_peers_response::GetSharedPeersResponse;
use crate::messages::{
    message::{MessagePayload, MessageType},
    requests::{
        announce_shard_request::AnnounceShardRequest,
        get_client_shard_info_request::GetClientShardInfoRequest,
        query_version_request::QueryVersionRequest, read_request::ReadRequest,
        write_request::WriteRequest,
    },
    responses::{
        announce_shard_response::AnnounceShardResponse,
        get_client_shard_info_response::GetClientShardInfoResponse,
        query_version_response::QueryVersionResponse, read_response::ReadResponse,
        write_response::WriteResponse,
    },
};
use anyhow::{anyhow, Ok, Result};
use async_recursion::async_recursion;
use std::future::IntoFuture;
use std::{cell::RefCell, sync::Arc};
use tokio::{
    io::{AsyncReadExt, Interest},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf, ReadHalf, WriteHalf},
        unix::SocketAddr,
        TcpListener, TcpStream,
    },
    task::JoinHandle,
};
use scc::HashMap;


use super::read::read_message;

/// Trait for handling callbacks to requests/responses from peers
pub trait RouterHandler: Send + Sync + 'static {
    /// Callback for handling new requests
    fn handle_announce_shard_request(&self, req: &AnnounceShardRequest) -> AnnounceShardResponse;

    fn handle_get_client_shard_info_request(
        &self,
        req: &GetClientShardInfoRequest,
    ) -> GetClientShardInfoResponse;

    fn handle_query_version_request(
        &self,
        req: &QueryVersionRequest,
    ) -> QueryVersionResponse;

    fn handle_read_request(&self, req: &ReadRequest) -> ReadResponse;

    fn handle_write_request(&self, req: &WriteRequest) -> WriteResponse;

    fn handle_get_shared_peers_request(&self, req: &GetSharedPeersRequest) -> GetSharedPeersResponse;

    /// Callbacks for handling responses to outbound requests
    fn handle_announce_shard_response(&self, res: &AnnounceShardResponse);

    fn handle_get_client_shard_info_response(&self, res: &GetClientShardInfoResponse);

    fn handle_query_version_response(&self, res: &QueryVersionResponse);

    fn handle_read_response(&self, res: &ReadResponse);

    fn handle_write_response(&self, res: &WriteResponse);

    fn handle_get_shared_peers_response(&self, res: &GetSharedPeersResponse);
}

pub struct RouterBuilder<H: RouterHandler>
{
    pub handler: Arc<H>,
    /// Map of peer addresses to write sockets
    pub write_sockets: Arc<HashMap<String, tokio::net::tcp::OwnedWriteHalf>>,

    pub bind_addr: Option<String>,
}

unsafe impl<H: RouterHandler> Send for RouterBuilder<H> {}

impl<H: RouterHandler> RouterBuilder<H> {
    pub fn new(handler: H, bind_addr: Option<String>) -> Self {
        Self {
            handler: Arc::new(handler),
            write_sockets: Arc::new(scc::HashMap::new()),
            bind_addr,
        }
    }

    /// Function for queueing outbound requests
    pub async fn queue_request<M: MessagePayload>(&self, req: M, peer: String) -> Result<()> {
        Self::create_write_socket_if_needed(self.write_sockets.clone(), self.handler.clone(), peer.clone()).await?;
        let mut write_socket = self.write_sockets.get_async(&peer).await.unwrap();
        write_message(&mut write_socket, Message {
           is_request: true,
           message_type: req.get_message_type(),
           message_payload: req
        }).await?;
        Ok(())
    }

    /// Function for queueing outbound responses
    async fn queue_response<M: MessagePayload>(write_sockets: Arc<HashMap<String, tokio::net::tcp::OwnedWriteHalf>>, handler: Arc<H>, res: M, peer: String) -> Result<()> {
        Self::create_write_socket_if_needed(write_sockets.clone(), handler.clone(), peer.clone()).await?;
        let mut write_socket = write_sockets.get_async(&peer).await.unwrap();
        write_message(&mut write_socket, Message {
           is_request: false,
           message_type: res.get_message_type(),
           message_payload: res
        }).await?;
        Ok(())
    }

    /// Creates a write socket for a peer if it doesn't exist
    /// I don't understand the async recursion problem but something to do with how async builds state machines
    /// https://www.reddit.com/r/rust/comments/kbu6bs/async_recursive_function_in_rust_using_futures/
    #[async_recursion]
    async fn create_write_socket_if_needed(write_sockets: Arc<HashMap<String, tokio::net::tcp::OwnedWriteHalf>>, handler: Arc<H>, peer: String) -> Result<()> {
        // check if peer is already connected
        if !write_sockets.contains_async(&peer).await {
            println!("creating!");
            let stream = TcpStream::connect(peer.clone()).await?;
            let (read, write) = stream.into_split();

            // push the write half to the map
            write_sockets.insert_async(peer.clone(), write).await.unwrap();

            // bind the read half to a background task
            let handler = handler.clone();
            let write_sockets = write_sockets.clone();
            tokio::spawn(async move {
                Self::listen_read_half_socket(write_sockets, handler, read).await?;
                Ok(())
            });
        }
        Ok(())
    }

    /// Listens for inbound requests on the read half of a socket from a peer
    async fn listen_read_half_socket(
        write_sockets: Arc<HashMap<String, tokio::net::tcp::OwnedWriteHalf>>,
        handler: Arc<H>,
        mut read: OwnedReadHalf,
    ) -> Result<()> {
        loop {
            println!("waiting for read");
            read.readable().await?;
            println!("done waiting for read");
            let message = read_message(&mut read).await?;
            let peer = read.peer_addr()?.to_string();

            println!("read message");
            match message.is_request() {
                true => {
                    match message.get_message_type() {
                        MessageType::AnnounceShard => {
                            let req = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<AnnounceShardRequest>()
                            .unwrap();
                            Self::queue_response::<AnnounceShardResponse>(write_sockets.clone(), handler.clone(), handler.handle_announce_shard_request(req), peer).await?;
                        }
                        MessageType::GetClientShardInfo => {
                            let req = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<GetClientShardInfoRequest>()
                            .unwrap();
                            Self::queue_response::<GetClientShardInfoResponse>(write_sockets.clone(), handler.clone(), handler.handle_get_client_shard_info_request(req), peer).await?;
                        }
                        MessageType::QueryVersion => {
                            let req = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<QueryVersionRequest>()
                            .unwrap();
                            Self::queue_response::<QueryVersionResponse>(write_sockets.clone(), handler.clone(), handler.handle_query_version_request(req), peer).await?;
                        }
                        MessageType::Read => {
                            let req = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<ReadRequest>()
                            .unwrap();
                            Self::queue_response::<ReadResponse>(write_sockets.clone(), handler.clone(), handler.handle_read_request(req), peer).await?;
                        }
                        MessageType::Write => {
                            let req = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<WriteRequest>()
                            .unwrap();
                            Self::queue_response::<WriteResponse>(write_sockets.clone(), handler.clone(), handler.handle_write_request(req), peer).await?;
                        }
                        MessageType::GetSharedPeers => {
                            let req = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<GetSharedPeersRequest>()
                            .unwrap();
                            Self::queue_response::<GetSharedPeersResponse>(write_sockets.clone(), handler.clone(), handler.handle_get_shared_peers_request(req), peer).await?;
                        }
                        MessageType::GetVersion => {
                            let req = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<QueryVersionRequest>()
                            .unwrap();
                            Self::queue_response::<QueryVersionResponse>(write_sockets.clone(), handler.clone(), handler.handle_query_version_request(req), peer).await?;
                        }
                    };
                }
                false => {
                    match message.get_message_type() {
                        MessageType::AnnounceShard => {
                            let res = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<AnnounceShardResponse>()
                            .unwrap();
                            handler.handle_announce_shard_response(res)
                        }
                        MessageType::GetClientShardInfo => {
                            let res = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<GetClientShardInfoResponse>()
                            .unwrap();
                            handler.handle_get_client_shard_info_response(res)
                        }
                        MessageType::QueryVersion => {
                            let res = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<QueryVersionResponse>()
                            .unwrap();
                            handler.handle_query_version_response(res)
                        }
                        MessageType::Read => {
                            let res = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<ReadResponse>()
                            .unwrap();
                            handler.handle_read_response(res)
                        }
                        MessageType::Write => {
                            let res = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<WriteResponse>()
                            .unwrap();
                            handler.handle_write_response(res)
                        }
                        MessageType::GetVersion => {
                            let res = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<QueryVersionResponse>()
                            .unwrap();
                            handler.handle_query_version_response(res)
                        }
                        MessageType::GetSharedPeers => {
                            let res = message
                            .as_ref()
                            .as_any()
                            .downcast_ref::<GetSharedPeersResponse>()
                            .unwrap();
                            handler.handle_get_shared_peers_response(res)
                        }   
                    }
                }
            };
        }
    }

    /// Makes the router start listening for inbound requests
    pub async fn listen(&self) -> Result<()> {
        let listener = TcpListener::bind(self.bind_addr.as_deref().unwrap_or("127.0.0.1:0")).await?;
        println!("listening on {}", listener.local_addr()?);

        loop {
            //let (mut socket, addr) = Arc::new(listener.accept().await?);
            let (socket, addr) = listener.accept().await?;
            print!("accepted connection!");

            let (read, write) = socket.into_split();

            // new peer discovered, add to our list of write sockets
            self.write_sockets
                .insert_async(addr.to_string(), write).await.unwrap();

            // bind the read half to a background task
            let handler = self.handler.clone();
            let write_sockets = self.write_sockets.clone();
            tokio::spawn(async move {
                Self::listen_read_half_socket(write_sockets, handler, read).await?;
                Ok(())
            });
        }
    }
}
