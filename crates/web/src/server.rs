use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
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
        // Plugin APIs
        .route("/api/plugins", get(list_plugins))
        .route("/api/plugins/{name}", get(get_plugin))
        .route("/api/plugins/{name}/execute", post(execute_plugin))
        .route("/api/plugins/{name}/reload", post(reload_plugin))
        .route("/api/plugins/upload", post(upload_plugin))
        .route("/ws/plugins/{name}", get(ws_handler))
        // Dashboard API
        .route("/api/dashboard", get(get_dashboard))
        // Docker APIs
        .route("/api/docker/containers", get(docker_containers))
        .route("/api/docker/images", get(docker_images))
        .route("/api/docker/containers/{id}/{action}", post(docker_action))
        .route("/api/docker/containers/{id}/logs", get(docker_logs))
        .route("/api/docker/containers/{id}/inspect", get(docker_inspect))
        .route("/api/docker/compose", get(docker_compose_list))
        .route("/api/docker/compose/{project}/{action}", post(docker_compose_action))
        .route("/api/docker/compose/{project}/logs", get(docker_compose_logs))
        // Middleware APIs
        .route("/api/middleware/config", get(mw_config))
        .route("/api/middleware/discover", get(mw_discover))
        .route("/api/middleware/redis/add", post(mw_redis_add))
        .route("/api/middleware/redis/remove", post(mw_redis_remove))
        .route("/api/middleware/redis/cli", post(mw_redis_cli))
        .route("/api/middleware/es/add", post(mw_es_add))
        .route("/api/middleware/es/remove", post(mw_es_remove))
        .route("/api/middleware/kafka/add", post(mw_kafka_add))
        .route("/api/middleware/kafka/remove", post(mw_kafka_remove))
        .route("/api/middleware/nginx/add", post(mw_nginx_add))
        .route("/api/middleware/nginx/remove", post(mw_nginx_remove))
        .route("/api/middleware/tomcat/add", post(mw_tomcat_add))
        .route("/api/middleware/tomcat/remove", post(mw_tomcat_remove))
        .route("/api/middleware/caddy/add", post(mw_caddy_add))
        .route("/api/middleware/caddy/remove", post(mw_caddy_remove))
        // WebSocket for dashboard
        .route("/ws/dashboard", get(ws_dashboard))
        .layer(cors)
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let local = listener.local_addr()?;
    println!(
        "\n{}\n  DevTool Web UI\n  {}\n  Local:   http://{}\n  Network: http://{}\n  Plugins: {} loaded\n{}",
        "\u{2550}".repeat(55),
        "\u{2500}".repeat(50),
        local,
        addr,
        pcount,
        "\u{2550}".repeat(55)
    );
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

fn json_error(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({"error": msg})))
}

// ============ Plugin APIs ============

#[derive(Serialize)]
struct PluginListResponse { plugins: Vec<PluginInfo> }

#[derive(Serialize)]
struct PluginInfo {
    name: String, version: String, description: String,
    category: String, actions: Vec<ActionInfo>,
}

#[derive(Serialize)]
struct ActionInfo { name: String, description: String }

async fn list_plugins(State(state): State<AppState>) -> Json<PluginListResponse> {
    let plugins: Vec<PluginInfo> = state.manager.list_plugins().into_iter()
        .map(|(meta, _)| PluginInfo {
            name: meta.name, version: meta.version, description: meta.description,
            category: meta.category.to_string(),
            actions: meta.actions.into_iter().map(|a| ActionInfo { name: a.name, description: a.description }).collect(),
        }).collect();
    Json(PluginListResponse { plugins })
}

async fn get_plugin(State(state): State<AppState>, Path(name): Path<String>) -> Result<Json<PluginInfo>, (StatusCode, Json<serde_json::Value>)> {
    state.manager.get_plugin(&name).map(|meta| Json(PluginInfo {
        name: meta.name, version: meta.version, description: meta.description,
        category: meta.category.to_string(),
        actions: meta.actions.into_iter().map(|a| ActionInfo { name: a.name, description: a.description }).collect(),
    })).ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Plugin not found"))
}

#[derive(Deserialize)]
struct ExecuteRequest {
    action: String, input_data: Option<String>, params: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
struct ExecuteResponse { success: bool, data: String, error: Option<String> }

async fn execute_plugin(State(state): State<AppState>, Path(name): Path<String>, Json(req): Json<ExecuteRequest>) -> Result<Json<ExecuteResponse>, (StatusCode, Json<serde_json::Value>)> {
    let input = PluginInput { action: req.action, params: req.params.unwrap_or_default(), input_data: req.input_data, input_file: None };
    state.manager.execute(&name, input).map(|output| Json(ExecuteResponse { success: output.success, data: output.data, error: output.error }))
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))
}

async fn reload_plugin(State(state): State<AppState>, Path(name): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.manager.reload(&name).map(|_| Json(serde_json::json!({"status": "reloaded"})))
        .map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))
}

async fn upload_plugin(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>, body: axum::body::Bytes) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let name = params.get("name").cloned().unwrap_or_else(|| "plugin.so".into());
    let safe_name = std::path::Path::new(&name).file_name().and_then(|n| n.to_str()).unwrap_or("plugin.so").to_string();
    if !safe_name.ends_with(".so") && !safe_name.ends_with(".dylib") && !safe_name.ends_with(".dll") {
        return Err(json_error(StatusCode::BAD_REQUEST, "Only .so/.dylib/.dll"));
    }
    let plugin_dir = state.manager.plugin_dir();
    let dest = plugin_dir.join(&safe_name);
    std::fs::write(&dest, &body).map_err(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    match state.manager.load(&dest) {
        Ok(meta) => Ok(Json(serde_json::json!({"status": "uploaded", "name": meta.name, "version": meta.version}))),
        Err(e) => Ok(Json(serde_json::json!({"status": "saved", "warning": format!("Saved but load failed: {}", e)}))),
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>, Path(name): Path<String>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state.manager, name))
}

async fn handle_ws(mut socket: axum::extract::ws::WebSocket, manager: PluginManager, plugin_name: String) {
    use axum::extract::ws::Message;
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            if let Ok(req) = serde_json::from_str::<ExecuteRequest>(&text) {
                let input = PluginInput { action: req.action, params: req.params.unwrap_or_default(), input_data: req.input_data, input_file: None };
                let response = match manager.execute(&plugin_name, input) {
                    Ok(output) => serde_json::to_string(&ExecuteResponse { success: output.success, data: output.data, error: output.error }).unwrap_or_default(),
                    Err(e) => serde_json::to_string(&ExecuteResponse { success: false, data: String::new(), error: Some(e.to_string()) }).unwrap_or_default(),
                };
                let _ = socket.send(Message::Text(response.into())).await;
            }
        }
    }
}

// ============ Dashboard API ============

async fn get_dashboard() -> Json<crate::dashboard::DashboardData> {
    Json(crate::dashboard::gather())
}

async fn ws_dashboard(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_ws_dashboard)
}

async fn handle_ws_dashboard(mut socket: axum::extract::ws::WebSocket) {
    use axum::extract::ws::Message;
    loop {
        let data = crate::dashboard::gather();
        let json = serde_json::to_string(&data).unwrap_or_default();
        if socket.send(Message::Text(json.into())).await.is_err() { break; }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

// ============ Docker APIs ============

async fn docker_containers() -> Json<serde_json::Value> {
    Json(serde_json::json!({"containers": crate::docker::list_containers()}))
}

async fn docker_images() -> Json<serde_json::Value> {
    Json(serde_json::json!({"images": crate::docker::list_images()}))
}

async fn docker_action(Path((id, action)): Path<(String, String)>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match crate::docker::container_action(&id, &action) {
        Ok(msg) => Ok(Json(serde_json::json!({"status": msg}))),
        Err(e) => Err(json_error(StatusCode::INTERNAL_SERVER_ERROR, &e)),
    }
}

async fn docker_logs(Path(id): Path<String>, Query(params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    let tail = params.get("tail").cloned().unwrap_or_else(|| "100".into());
    Json(serde_json::json!({"logs": crate::docker::container_logs(&id, &tail)}))
}

async fn docker_inspect(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"inspect": crate::docker::container_inspect(&id)}))
}

// ============ Middleware APIs ============

async fn mw_config() -> Json<crate::middleware::MiddlewareConfig> {
    Json(crate::middleware::load_config())
}

async fn mw_discover() -> Json<serde_json::Value> {
    Json(serde_json::json!({"services": crate::middleware::discover_services()}))
}

#[derive(Deserialize)]
struct RedisAddReq { name: String, host: String, port: u16, password: Option<String>, db: Option<u16> }

async fn mw_redis_add(Json(req): Json<RedisAddReq>) -> Json<serde_json::Value> {
    crate::middleware::add_redis_conn(crate::middleware::RedisConn { name: req.name, host: req.host, port: req.port, password: req.password, db: req.db.unwrap_or(0) });
    Json(serde_json::json!({"status": "ok"}))
}

#[derive(Deserialize)]
struct NameReq { name: String }

async fn mw_redis_remove(Json(req): Json<NameReq>) -> Json<serde_json::Value> {
    crate::middleware::remove_redis_conn(&req.name);
    Json(serde_json::json!({"status": "ok"}))
}

#[derive(Deserialize)]
struct RedisCliReq { host: String, port: u16, password: Option<String>, cmd: String }

async fn mw_redis_cli(Json(req): Json<RedisCliReq>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match crate::middleware::redis_cli(&req.host, req.port, &req.password, &req.cmd) {
        Ok(output) => Ok(Json(serde_json::json!({"output": output}))),
        Err(e) => Err(json_error(StatusCode::INTERNAL_SERVER_ERROR, &e)),
    }
}

#[derive(Deserialize)]
struct EsAddReq { name: String, host: String, port: u16, user: Option<String>, password: Option<String>, scheme: Option<String> }

async fn mw_es_add(Json(req): Json<EsAddReq>) -> Json<serde_json::Value> {
    crate::middleware::add_es_conn(crate::middleware::EsConn { name: req.name, host: req.host, port: req.port, user: req.user, password: req.password, scheme: req.scheme.unwrap_or_else(|| "http".into()) });
    Json(serde_json::json!({"status": "ok"}))
}

async fn mw_es_remove(Json(req): Json<NameReq>) -> Json<serde_json::Value> {
    crate::middleware::remove_es_conn(&req.name);
    Json(serde_json::json!({"status": "ok"}))
}

#[derive(Deserialize)]
struct KafkaAddReq { name: String, brokers: String, sasl_user: Option<String>, sasl_password: Option<String> }

async fn mw_kafka_add(Json(req): Json<KafkaAddReq>) -> Json<serde_json::Value> {
    crate::middleware::add_kafka_conn(crate::middleware::KafkaConn { name: req.name, brokers: req.brokers, sasl_user: req.sasl_user, sasl_password: req.sasl_password });
    Json(serde_json::json!({"status": "ok"}))
}

async fn mw_kafka_remove(Json(req): Json<NameReq>) -> Json<serde_json::Value> {
    crate::middleware::remove_kafka_conn(&req.name);
    Json(serde_json::json!({"status": "ok"}))
}

#[derive(Deserialize)]
struct NginxAddReq { name: String, config_path: Option<String>, log_path: Option<String>, pid_path: Option<String> }

async fn mw_nginx_add(Json(req): Json<NginxAddReq>) -> Json<serde_json::Value> {
    crate::middleware::add_nginx_conn(crate::middleware::NginxConn { name: req.name, config_path: req.config_path, log_path: req.log_path, pid_path: req.pid_path });
    Json(serde_json::json!({"status": "ok"}))
}

async fn mw_nginx_remove(Json(req): Json<NameReq>) -> Json<serde_json::Value> {
    crate::middleware::remove_nginx_conn(&req.name);
    Json(serde_json::json!({"status": "ok"}))
}

#[derive(Deserialize)]
struct TomcatAddReq { name: String, catalina_home: Option<String>, log_path: Option<String>, pid_path: Option<String> }

async fn mw_tomcat_add(Json(req): Json<TomcatAddReq>) -> Json<serde_json::Value> {
    crate::middleware::add_tomcat_conn(crate::middleware::TomcatConn { name: req.name, catalina_home: req.catalina_home, log_path: req.log_path, pid_path: req.pid_path });
    Json(serde_json::json!({"status": "ok"}))
}

async fn mw_tomcat_remove(Json(req): Json<NameReq>) -> Json<serde_json::Value> {
    crate::middleware::remove_tomcat_conn(&req.name);
    Json(serde_json::json!({"status": "ok"}))
}

#[derive(Deserialize)]
struct CaddyAddReq { name: String, config_path: Option<String>, log_path: Option<String> }

async fn mw_caddy_add(Json(req): Json<CaddyAddReq>) -> Json<serde_json::Value> {
    crate::middleware::add_caddy_conn(crate::middleware::CaddyConn { name: req.name, config_path: req.config_path, log_path: req.log_path });
    Json(serde_json::json!({"status": "ok"}))
}

async fn mw_caddy_remove(Json(req): Json<NameReq>) -> Json<serde_json::Value> {
    crate::middleware::remove_caddy_conn(&req.name);
    Json(serde_json::json!({"status": "ok"}))
}

// ============ Docker Compose APIs ============

async fn docker_compose_list() -> Json<serde_json::Value> {
    Json(serde_json::json!({"projects": crate::docker::list_compose_projects()}))
}

async fn docker_compose_action(Path((project, action)): Path<(String, String)>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match crate::docker::compose_action(&project, &action) {
        Ok(output) => Ok(Json(serde_json::json!({"status": "ok", "output": output}))),
        Err(e) => Err(json_error(StatusCode::INTERNAL_SERVER_ERROR, &e)),
    }
}

async fn docker_compose_logs(Path(project): Path<String>, Query(params): Query<std::collections::HashMap<String, String>>) -> Json<serde_json::Value> {
    let tail = params.get("tail").cloned().unwrap_or_else(|| "100".into());
    Json(serde_json::json!({"logs": crate::docker::compose_logs(&project, &tail)}))
}
