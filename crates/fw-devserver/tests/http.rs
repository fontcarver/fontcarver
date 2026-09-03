use std::{net::SocketAddr, sync::Arc};

use futures_util::StreamExt;
use fw_api::Api;
use fw_devserver::ServeOptions;
use fw_doc::Session;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::{net::TcpListener, task::JoinHandle, time::Duration};
use tokio_tungstenite::connect_async;

struct TestServer {
    address: SocketAddr,
    task: JoinHandle<()>,
    _config_dir: TempDir,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn_server() -> TestServer {
    let config_dir = TempDir::new().expect("temporary config directory");
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind random port");
    let address = listener.local_addr().expect("listener address");
    let api = Arc::new(Api::new(Arc::new(Session::new())));
    let options = ServeOptions {
        config_dir: Some(config_dir.path().to_owned()),
    };
    let task = tokio::spawn(async move {
        fw_devserver::serve(listener, api, options)
            .await
            .expect("serve API");
    });
    TestServer {
        address,
        task,
        _config_dir: config_dir,
    }
}

async fn post(client: &Client, address: SocketAddr, method: &str, params: Value) -> Value {
    client
        .post(format!("http://{address}/api/{method}"))
        .json(&params)
        .send()
        .await
        .expect("send API request")
        .error_for_status()
        .expect("successful API status")
        .json()
        .await
        .expect("JSON API response")
}

#[tokio::test]
async fn serves_http_errors_and_document_events() {
    let server = spawn_server().await;
    let address = server.address;
    let client = Client::new();

    let opened = post(
        &client,
        address,
        "session.open",
        json!({
            "path": fw_fixtures::path("DejaVuSans.ttf"),
            "face_index": null
        }),
    )
    .await;
    assert_eq!(opened["result"]["glyph_count"], 6_253);
    let doc = opened["result"]["doc"].clone();

    let missing = client
        .post(format!("http://{address}/api/nope"))
        .json(&json!({}))
        .send()
        .await
        .expect("send unknown-method request");
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        missing.json::<Value>().await.expect("JSON error")["error"]["code"],
        "not_found"
    );

    let glyphs = post(
        &client,
        address,
        "glyphs.search",
        json!({ "doc": doc, "query": "A", "limit": 10 }),
    )
    .await;
    let glyph = glyphs["result"]
        .as_array()
        .and_then(|glyphs| glyphs.iter().find(|glyph| glyph["name"] == "A"))
        .expect("A glyph");
    let detail = post(
        &client,
        address,
        "glyphs.detail",
        json!({ "doc": doc, "gid": glyph["gid"] }),
    )
    .await;
    let layer = detail["result"]["glyph"]["layers"][0]["id"].clone();
    let (mut events, _) = connect_async(format!("ws://{address}/events"))
        .await
        .expect("connect event WebSocket");
    post(
        &client,
        address,
        "doc.apply",
        json!({
            "doc": doc,
            "command": { "SetWidth": { "glyph": "A", "layer": layer, "width": 777.0 } },
            "label": "Set width"
        }),
    )
    .await;

    let message = tokio::time::timeout(Duration::from_secs(5), events.next())
        .await
        .expect("event timeout")
        .expect("event stream ended")
        .expect("WebSocket event");
    let event: Value =
        serde_json::from_str(message.to_text().expect("text event")).expect("document event JSON");
    assert_eq!(event["doc"], doc);
    assert_eq!(event["generation"], 1);
    assert_eq!(event["dirty"], true);
}

#[tokio::test]
async fn concurrent_opens_do_not_block_each_other_or_health() {
    let server = spawn_server().await;
    let address = server.address;
    let client = Client::new();
    let font = fw_fixtures::external("source-han-sans-sc-otf")
        .unwrap_or_else(|| fw_fixtures::path("DejaVuSans.ttf"));
    let params = json!({ "path": font, "face_index": null });

    let first = tokio::spawn({
        let client = client.clone();
        let params = params.clone();
        async move { post(&client, address, "session.open", params).await }
    });
    let second = tokio::spawn({
        let client = client.clone();
        async move { post(&client, address, "session.open", params).await }
    });

    tokio::time::sleep(Duration::from_millis(25)).await;
    assert!(
        !first.is_finished() || !second.is_finished(),
        "an open request must be in flight before checking health"
    );
    let health = tokio::time::timeout(
        Duration::from_millis(500),
        client.get(format!("http://{address}/health")).send(),
    )
    .await
    .expect("health response while opening")
    .expect("send health request");
    assert_eq!(health.status(), StatusCode::OK);
    assert!(
        !first.is_finished() || !second.is_finished(),
        "an open request must still be in flight when health answers"
    );

    let (first, second) = tokio::join!(first, second);
    assert!(first.expect("first open task")["result"]["glyph_count"].is_number());
    assert!(second.expect("second open task")["result"]["glyph_count"].is_number());
}
