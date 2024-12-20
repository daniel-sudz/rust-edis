#[cfg(test)]
mod tests {
    use anyhow::Result;
    use assert_cmd::prelude::*;

    use predicates::prelude::*;
    use serial_test::serial;

    use std::process::Command;

    use crate::integration::test_setup;
    use crate::messages::requests::get_client_shard_info_request::GetClientShardInfoRequest;

    use crate::utils::constants::MAIN_INSTANCE_IP_PORT;
    use crate::utils::test_client;
    use std::process::Stdio;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    #[serial]
    async fn test_basic_integration() -> Result<()> {
        let r: Result<()> = {
            test_setup::setup_test().await;

            // start everything
            let mut info_cmd = Command::cargo_bin("info")?;
            let mut read_cmd = Command::cargo_bin("read_shard")?;
            let mut write_cmd = Command::cargo_bin("write_shard")?;

            // the output is really noisy unless we need it for debugging
            info_cmd
                .arg("--write-shards=1")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            read_cmd
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            write_cmd
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            // get the client shard into list
            let test_client = test_client::TestRouterClient::new();
            test_client
                .get_client()
                .queue_request(GetClientShardInfoRequest {}, MAIN_INSTANCE_IP_PORT)
                .await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let client_shard_info_responses =
                test_client.get_client_shard_info_responses.lock().unwrap();
            assert_eq!(client_shard_info_responses.len(), 1);
            assert_eq!(client_shard_info_responses[0].num_write_shards, 1);
            assert_eq!(client_shard_info_responses[0].write_shard_info.len(), 1);
            assert_eq!(client_shard_info_responses[0].read_shard_info.len(), 1);

            Ok(())
        };

        test_setup::test_teardown().await;
        r
    }

    #[tokio::test]
    #[serial]
    async fn test_set_command_integration() -> Result<()> {
        // Start the info server
        let mut info_server = Command::cargo_bin("info")?;
        info_server
            .arg("--write-shards=1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        // Start the write shard server
        let mut write_server = Command::cargo_bin("write_shard")?;
        write_server
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        // Start the read shard server
        let mut read_server = Command::cargo_bin("read_shard")?;
        read_server
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        // Allow servers to initialize
        sleep(Duration::from_secs(5)).await;

        // Test the client binary with a set command
        let mut client1 = assert_cmd::Command::cargo_bin("client")?;
        let mut client2 = assert_cmd::Command::cargo_bin("client")?;

        sleep(Duration::from_secs(1)).await;
        client1
            .write_stdin("set test_key test_value\nwait\nexit\n")
            .assert()
            .stdout(predicate::str::contains("OK"));

        sleep(Duration::from_secs(1)).await;
        client2
            .write_stdin("get test_key\nwait\nexit\n")
            .assert()
            .stdout(predicate::str::contains("test_key"));

        Ok(())
    }
}
