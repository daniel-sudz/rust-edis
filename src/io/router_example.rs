use crate::{messages::{requests::{announce_shard_request::AnnounceShardRequest, get_client_shard_info_request::GetClientShardInfoRequest, query_version_request::QueryVersionRequest, read_request::ReadRequest, write_request::WriteRequest}, responses::{announce_shard_response::AnnounceShardResponse, get_client_shard_info_response::GetClientShardInfoResponse, query_version_response::QueryVersionResponse, read_response::ReadResponse, write_response::WriteResponse}}};
use crate::io::router::{RouterBuilder, RouterHandler};


struct ExampleRouterHandler {

}

impl RouterHandler for ExampleRouterHandler {
    /// Callback for handling new requests
    fn handle_announce_shard_request(&self, req: &AnnounceShardRequest) -> AnnounceShardResponse {
        AnnounceShardResponse {
            writer_number: 0,   
        }
    }

    fn handle_get_client_shard_info_request(
        req: GetClientShardInfoRequest,
    ) -> GetClientShardInfoResponse {
        GetClientShardInfoResponse {
            num_write_shards: 0,
            write_shard_info: vec![],
            read_shard_info: vec![],
        }
    }

    fn handle_query_version_request(req: QueryVersionRequest) -> QueryVersionResponse {
        QueryVersionResponse {
            version: 0,
        }
    }

    fn handle_read_request(req: ReadRequest) -> ReadResponse {
        ReadResponse {
            value: vec![],
        }
    }

    fn handle_write_request(req: WriteRequest) -> WriteResponse {
        WriteResponse {
            error: 0,
        }
    }

    /// Callbacks for handling responses to outbound requests
    fn handle_announce_shard_response(res: AnnounceShardResponse) {

    }

    fn handle_get_client_shard_info_response(res: GetClientShardInfoResponse) {
        
    }

    fn handle_query_version_response(res: QueryVersionResponse) {

    }

    fn handle_read_response(res: ReadResponse) {

    }

    fn handle_write_response(res: WriteResponse) {

    }
}


mod test {
    use std::cell::Cell;

    use crate::{io::{router::RouterBuilder, router_example::ExampleRouterHandler}, messages::requests::query_version_request::QueryVersionRequest};


    #[tokio::test]
    async fn test_example_router() {
        let router1 = RouterBuilder::new(ExampleRouterHandler {}, Some("127.0.0.1:8080".to_string()));
        let router2 = RouterBuilder::new(ExampleRouterHandler {}, Some("127.0.0.1:8081".to_string()));

        tokio::spawn(async move {
            router1.listen().await
        });

        tokio::spawn(async move {
            router2.listen().await
        });

    }
}