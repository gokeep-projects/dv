use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use devtool_core::manager::PluginManager;
use devtool_core::types::PluginInput;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

#[derive(RustEmbed)]
#[folder = "../../assets/web/"]
struct WebAssets;

#[derive(Clone)]
struct AppState {
    manager: PluginManager,
}

pub async fn run(
    manager: PluginManager,
    host: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = manager.load_all();
    let pcount = manager.plugin_count();
    let state = AppState { manager };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(index))
        .route("/assets/{*path}", get(serve_asset))
        .route("/api/plugins", get(list_plugins))
        .route("/api/plugins/{name}", get(get_plugin))
        .route("/api/plugins/{name}/execute", post(execute_plugin))
        .route("/api/plugins/{name}/reload", post(reload_plugin))
        .route("/api/plugins/upload", post(upload_plugin))
        .route("/ws/plugins/{name}", get(ws_handler))
        .layer(cors)
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let local = listener.local_addr()?;
    println!("\n{}\n  DevTool Web UI\n  {}\n  Local:   http://{}\n  Network: http://{}\n  Plugins: {} loaded\n{}",
        "\u{2550}".repeat(55), "\u{2500}".repeat(50), local, addr, pcount, "\u{2550}".repeat(55));
    info!("Web UI on http://{}", local);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index() -> impl IntoResponse {
    match WebAssets::get("index.html") {
        Some(content) => Response::builder()
            .header("content-type", "text/html; charset=utf-8")
            .body(axum::body::Body::from(content.data))
            .unwrap(),
        None => (StatusCode::NOT_FOUND, "index.html not found").into_response(),
    }
}

async fn serve_asset(Path(path): Path<String>) -> impl IntoResponse {
    match WebAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            Response::builder()
                .header("content-type", mime.as_ref())
                .body(axum::body::Body::from(content.data))
                .unwrap()
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

#[derive(Serialize)]
struct PluginListResponse {
    plugins: Vec<PluginInfo>,
}

#[derive(Serialize)]
struct PluginInfo {
    name: String,
    version: String,
    description: String,
    category: String,
    actions: Vec<ActionInfo>,
}

#[derive(Serialize)]
struct ActionInfo {
    name: String,
    description: String,
}

async fn list_plugins(State(state): State<AppState>) -> Json<PluginListResponse> {
    let plugins: Vec<PluginInfo> = state
        .manager
        .list_plugins()
        .into_iter()
        .map(|(meta, _)| PluginInfo {
            name: meta.name,
            version: meta.version,
            description: meta.description,
            category: meta.category.to_string(),
            actions: meta
                .actions
                .into_iter()
                .map(|a| ActionInfo {
                    name: a.name,
                    description: a.description,
                })
                .collect(),
        })
        .collect();

    Json(PluginListResponse { plugins })
}

fn json_error(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({"error": msg})))
}

async fn get_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<PluginInfo>, (StatusCode, Json<serde_json::Value>)> {
    state
        .manager
        .get_plugin(&name)
        .map(|meta| {
            Json(PluginInfo {
                name: meta.name,
                version: meta.version,
                description: meta.description,
                category: meta.category.to_string(),
                actions: meta
                    .actions
                    .into_iter()
                    .map(|a| ActionInfo {
                        name: a.name,
                        description: a.description,
                    })
                    .collect(),
            })
        })
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Plugin not found"))
}

#[derive(Deserialize)]
struct ExecuteRequest {
    action: String,
    input_data: Option<String>,
    params: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
struct ExecuteResponse {
    success: bool,
    data: String,
    error: Option<String>,
}

async fn execute_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResponse>, (StatusCode, Json<serde_json::Value>)> {
    let input = PluginInput {
        action: req.action,
        params: req.params.unwrap_or_default(),
        input_data: req.input_data,
        input_file: None,
    };

    state
        .manager
        .execute(&name, input)
        .map(|output| {
            Json(ExecuteResponse {
                success: output.success,
                data: output.data,
                error: output.error,
            })
        })
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))
}

async fn reload_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .manager
        .reload(&name)
        .map(|_| Json(serde_json::json!({"status": "reloaded"})))
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))
}

use axum::extract::Query;

async fn upload_plugin(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let name = params.get("name").cloned().unwrap_or_else(|| "plugin.so".into());
    // Sanitize: remove path traversal
    let safe_name = std::path::Path::new(&name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("plugin.so")
        .to_string();

    if !safe_name.ends_with(".so") && !safe_name.ends_with(".dylib") && !safe_name.ends_with(".dll") {
        return Err(json_error(StatusCode::BAD_REQUEST, "Only .so / .dylib / .dll files allowed"));
    }

    let plugin_dir = state.manager.plugin_dir();
    let dest = plugin_dir.join(&safe_name);
    std::fs::write(&dest, &body)
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    // Try to load it
    match state.manager.load(&dest) {
        Ok(meta) => Ok(Json(serde_json::json!({
            "status": "uploaded",
            "name": meta.name,
            "version": meta.version,
        }))),
        Err(e) => {
            // Still saved, just couldn't load
            Ok(Json(serde_json::json!({
                "status": "saved",
                "warning": format!("File saved but could not load: {}", e),
            })))
        }
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state.manager, name))
}

async fn handle_ws(
    mut socket: axum::extract::ws::WebSocket,
    manager: PluginManager,
    plugin_name: String,
) {
    use axum::extract::ws::Message;

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            if let Ok(req) = serde_json::from_str::<ExecuteRequest>(&text) {
                let input = PluginInput {
                    action: req.action,
                    params: req.params.unwrap_or_default(),
                    input_data: req.input_data,
                    input_file: None,
                };

                let result = manager.execute(&plugin_name, input);
                let response = match result {
                    Ok(output) => serde_json::to_string(&ExecuteResponse {
                        success: output.success,
                        data: output.data,
                        error: output.error,
                    })
                    .unwrap_or_default(),
                    Err(e) => serde_json::to_string(&ExecuteResponse {
                        success: false,
                        data: String::new(),
                        error: Some(e.to_string()),
                    })
                    .unwrap_or_default(),
                };

                let _ = socket.send(Message::Text(response.into())).await;
            }
        }
    }
}
