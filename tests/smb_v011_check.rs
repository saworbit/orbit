#[cfg(feature = "smb-native")]
mod smb_v011_tests {
    use futures::StreamExt;
    use orbit::backend::{Backend, SmbBackend, SmbConfig};
    use orbit::protocols::smb::SmbSecurity;
    use std::path::Path;

    // REQUIRES: Docker container running samba on port 4455
    // docker run -d -p 4455:445 -e USERNAME=test -e PASSWORD=test dperson/samba -s "public;/share;yes;no;no;test"

    #[tokio::test]
    #[ignore]
    async fn test_v011_query_and_io() {
        let config = SmbConfig::new("localhost", "public")
            .with_port(4455)
            .with_username("test")
            .with_password("test")
            .with_security(SmbSecurity::Opportunistic);

        let backend = SmbBackend::new(config).await.expect("Failed to connect");

        // 1. Write
        let content = b"Orbit v0.11.0 Test";
        backend
            .write(
                Path::new("test.txt"),
                Box::new(&content[..]),
                Some(content.len() as u64),
                Default::default(),
            )
            .await
            .expect("Write failed");

        // 2. List (Tests the new `query` API)
        let mut list_stream = backend
            .list(Path::new(""), Default::default())
            .await
            .expect("List failed");
        let mut found = false;
        while let Some(entry) = list_stream.next().await {
            let e = entry.expect("Entry error");
            if e.path.to_string_lossy() == "test.txt" {
                found = true;
            }
        }
        assert!(found, "Did not find test.txt in directory listing");

        // 3. Read
        let mut read_stream = backend
            .read(Path::new("test.txt"))
            .await
            .expect("Read failed");
        let chunk = read_stream
            .next()
            .await
            .expect("Stream empty")
            .expect("Read error");
        assert_eq!(chunk, content);

        // 4. Cleanup
        backend
            .delete(Path::new("test.txt"), false)
            .await
            .expect("Delete failed");
    }
}
