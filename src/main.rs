mod kroki;
mod mcp;

use axum::{
    extract::State,
    http::{StatusCode, HeaderMap},
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::broadcast;
use futures_util::stream::{self, Stream};
use tracing::{info, warn, error, debug};
use tower_http::trace::TraceLayer;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// --- Structures MCP minimales pour le parsing interne ---
#[derive(Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct McpResponse {
    jsonrpc: String,
    id: Value,
    result: Value,
}

struct AppState {
    kroki_url: String,
    tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    // Logging au format compact/syslog comme convenu
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .with_ansi(false)
        .with_timer(tracing_subscriber::fmt::time::ChronoUtc::rfc_3339())
        .compact()
        .init();

    let (tx, _) = broadcast::channel(100);
    let kroki_url = std::env::var("MCP_KROKI_URL").unwrap_or_else(|_| "https://kroki.io".to_string());
    let port = std::env::var("MCP_KROKI_PORT").unwrap_or_else(|_| "3001".to_string());

   let state = Arc::new(AppState { kroki_url, tx });

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/sse", get(sse_handler).post(messages_handler))
        .route("/messages", post(messages_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::clone(&state)); // On clone ici (ou state.clone())

    let addr = format!("0.0.0.0:{}", port);

    info!(msg = "🚀 MCP Kroki Bridge started", addr = %addr, upstream = %state.kroki_url);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Handler du tunnel SSE (GET /sse)
async fn sse_handler(State(state): State<Arc<AppState>>) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(msg) => Some((Ok(Event::default().data(msg)), rx)),
            Err(_) => None,
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::new())
}

// Handler principal des messages (POST /messages ou POST /sse)
async fn messages_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<McpRequest>,
) -> impl IntoResponse {
    let tx = state.tx.clone();
    let method = payload.method.clone();
    let request_id = payload.id.clone().unwrap_or(Value::Null);

    // --- STRATÉGIE HYBRIDE : Initialisation directe ---
    if method == "initialize" {
        info!(method = "initialize", msg = "Handling via direct HTTP response");
        let result = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "mcp-kroki-bridge", "version": "0.1.0" }
        });
        let response = McpResponse { jsonrpc: "2.0".into(), id: request_id, result };
        return (StatusCode::OK, Json(response)).into_response();
    }

    // --- Traitement asynchrone pour les outils ---
    tokio::spawn(async move {
        // Ignorer les notifications sans ID
        if request_id.is_null() && method != "notifications/initialized" { return; }

        let result = match method.as_str() {
            "tools/list" => {
                debug!(method = "tools/list", msg = "Providing tool definitions");
                mcp::get_tools_list()
            },
            "tools/call" => {
                let tool_name = payload.params.as_ref().and_then(|p| p.get("name")?.as_str()).unwrap_or("");
                let args = payload.params.as_ref().and_then(|p| p.get("arguments"));
                let source = args.and_then(|a| a.get("source")?.as_str()).unwrap_or("");

                info!(method = "tools/call", tool = %tool_name, src_len = source.len());

                let kroki_type = match tool_name {
                    "render_plantuml" => "plantuml",
                    "render_vega" => "vegalite",
                    "render_mermaid" => "mermaid",
                    _ => "mermaid",
                };

                let url = kroki::generate_url(&state.kroki_url, kroki_type, source);
                
                json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Diagramme généré !\n\n![Diagram]({})\n\nLien direct : {}", url, url)
                    }]
                })
            },
            "notifications/initialized" => {
                info!(method = "notifications/initialized", status = "ready");
                return;
            },
            _ => json!({ "isError": true, "content": [{ "type": "text", "text": format!("Method {} not supported", method) }] }),
        };

        // Envoi de la réponse via le tunnel SSE
        let response = McpResponse { jsonrpc: "2.0".into(), id: request_id, result };
        if let Ok(json_msg) = serde_json::to_string(&response) {
            let mut delivered = false;
            for _ in 0..3 {
                if tx.send(json_msg.clone()).is_ok() {
                    delivered = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            if !delivered {
                warn!(method = %method, msg = "Could not deliver via SSE");
            }
        }
    });

    StatusCode::ACCEPTED.into_response()
}