//! Axum routes for the browser-development transport.

use std::{path::PathBuf, sync::Arc};

use axum::{
    extract::{
        rejection::JsonRejection,
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    http::{header::CONTENT_TYPE, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use fc_api::{Api, ApiError};
use serde::Serialize;
use serde_json::Value;
use tokio::{net::TcpListener, sync::broadcast::error::RecvError};
use tower_http::cors::CorsLayer;

use crate::Error;

#[derive(Serialize)]
struct ResultEnvelope {
    result: Value,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ApiError,
}

/// Runtime configuration for the development server.
#[derive(Clone, Debug, Default)]
pub struct ServeOptions {
    /// Overrides the FontCarver configuration directory for this server.
    pub config_dir: Option<PathBuf>,
}

/// Builds the HTTP and WebSocket router backed by `api`.
pub fn router(api: Arc<Api>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:1420"),
            HeaderValue::from_static("http://127.0.0.1:1420"),
        ])
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE]);

    Router::new()
        .route("/health", get(health))
        .route("/api/{method}", post(api_call))
        .route("/events", get(events))
        .layer(cors)
        .with_state(api)
}

/// Serves the development API until the listener fails or the task is cancelled.
///
/// # Errors
///
/// Returns an error when Axum cannot serve a connection on `listener`.
pub async fn serve(
    listener: TcpListener,
    api: Arc<Api>,
    options: ServeOptions,
) -> Result<(), Error> {
    if let Some(config_dir) = options.config_dir {
        api.configure_config_dir(config_dir)?;
    }
    axum::serve(listener, router(api))
        .await
        .map_err(Error::from)
}

async fn health() -> Json<Value> {
    Json(serde_json::json!({ "ok": true }))
}

async fn api_call(
    State(api): State<Arc<Api>>,
    Path(method): Path<String>,
    params: Result<Json<Value>, JsonRejection>,
) -> Response {
    let params = match params {
        Ok(Json(params)) => params,
        Err(error) => {
            return api_error_response(ApiError {
                code: "invalid_params".to_owned(),
                message: error.body_text(),
            });
        }
    };

    let call = tokio::task::spawn_blocking(move || api.call(&method, params)).await;
    match call {
        Ok(Ok(result)) => (StatusCode::OK, Json(ResultEnvelope { result })).into_response(),
        Ok(Err(error)) => api_error_response(error),
        Err(error) => api_error_response(ApiError {
            code: "internal".to_owned(),
            message: format!("API worker task failed: {error}"),
        }),
    }
}

fn api_error_response(error: ApiError) -> Response {
    let status = match error.code.as_str() {
        "not_found" => StatusCode::NOT_FOUND,
        "invalid_params" => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(ErrorEnvelope { error })).into_response()
}

async fn events(State(api): State<Arc<Api>>, upgrade: WebSocketUpgrade) -> Response {
    let events = api.subscribe();
    upgrade.on_upgrade(move |socket| event_stream(socket, events))
}

async fn event_stream(
    mut socket: WebSocket,
    mut events: tokio::sync::broadcast::Receiver<fc_api::Event>,
) {
    loop {
        tokio::select! {
            event = events.recv() => {
                let event = match event {
                    Ok(event) => event,
                    Err(RecvError::Lagged(skipped)) => {
                        tracing::warn!(skipped, "WebSocket event subscriber lagged");
                        continue;
                    }
                    Err(RecvError::Closed) => break,
                };
                let text = match serde_json::to_string(&event) {
                    Ok(text) => text,
                    Err(error) => {
                        tracing::error!(%error, "failed to serialize kernel event");
                        continue;
                    }
                };
                if socket.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
            message = socket.recv() => {
                match message {
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => {}
                }
            }
        }
    }
}
