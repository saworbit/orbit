#![cfg(feature = "gui")]

use std::net::TcpListener;
use std::time::Duration;

#[tokio::test]
#[ignore = "Requires Leptos server assets; skip in default test runs"]
async fn orbit_web_health_endpoint_responds() {
    // Reserve an ephemeral port and release it so the server can bind to the same address.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");
    drop(listener);

    let config = orbit_web::WebConfig {
        host: addr.ip().to_string(),
        port: addr.port(),
        magnetar_db: "test-magnetar.db".to_string(),
        user_db: "test-users.db".to_string(),
    };

    let server = tokio::spawn(orbit_web::start_server(config));

    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = reqwest::get(format!("http://{}/api/health", addr))
        .await
        .expect("request health endpoint");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("parse json");
    assert_eq!(
        body.get("status"),
        Some(&serde_json::Value::String("ok".into()))
    );

    server.abort();
}
