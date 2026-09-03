use std::{net::SocketAddr, sync::Arc};

use futures_util::StreamExt;
use fw_api::Api;
use fw_doc::Session;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use tokio::{net::TcpListener, task::JoinHandle, time::Duration};
use tokio_tungstenite::connect_async;

async fn spawn_server() -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind random port");
    let address = listener.local_addr().expect("listener address");
    let api = Arc::new(Api::new(Arc::new(Session::new())));
    let task = tokio::spawn(async move {
        fw_devserver::serve(listener, api).await.expect("serve API");
    });
    (address, task)
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
    let config_dir = std::env::temp_dir().join(format!(
        "fontworkshop-fw-devserver-http-{}",
        std::process::id()
    ));
    std::env::set_var("FW_CONFIG_DIR", &config_dir);
    let (address, server) = spawn_server().await;
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

    let (mut events, _) = connect_async(format!("ws://{address}/events"))
        .await
        .expect("connect event WebSocket");
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

    server.abort();
    std::fs::remove_dir_all(config_dir).ok();
}
