#[cfg(test)]
mod tests {
    use anyhow::Result;
    use assert_cmd::prelude::*;
    use port_killer::{kill, kill_by_pids};
    use predicates::prelude::*;
    use serial_test::serial;
    use std::path::{absolute, Path};
    use std::process::Command;

    use crate::integration::test_setup;
    use crate::messages::requests::get_client_shard_info_request::GetClientShardInfoRequest;
    use crate::messages::responses::get_client_shard_info_response;
    use crate::utils::constants::MAIN_INSTANCE_IP_PORT;
    use crate::utils::test_client;
    use std::process::Stdio;
    use tokio::time::{sleep, Duration};

    #[serial]
    #[tokio::test]
    async fn test_basic_integration() -> Result<()> {
        let r: Result<()> = {
            test_setup::setup_test();

            // start everything
            let mut info_cmd = Command::cargo_bin("info")?;
            let mut read_cmd = Command::cargo_bin("read_shard")?;
            let mut write_cmd = Command::cargo_bin("write_shard")?;

            // the output is really noisy unless we need it for debugging
            info_cmd
                .arg("--write-shards=1")
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()?;
            read_cmd
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()?;
            write_cmd
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
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

        test_setup::test_teardown();
        r
    }

    #[tokio::test]
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
        let mut client = assert_cmd::Command::cargo_bin("client")?;

        sleep(Duration::from_secs(1)).await;
        client
            .write_stdin("set test_key test_value\n")
            .assert()
            .stdout(predicate::str::contains("OK"));

        Ok(())
    }
}