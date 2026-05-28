use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;

struct HttpClientPlugin;

impl Plugin for HttpClientPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "http-client".into(),
            version: "0.1.0".into(),
            description: "HTTP client for API testing: GET, POST, PUT, DELETE with headers and body".into(),
            author: "DevTool Team".into(),
            category: PluginCategory::Network,
            actions: vec![
                PluginAction {
                    name: "get".into(), description: "Send HTTP GET request".into(),
                    params: vec![
                        ActionParam { name: "url".into(), description: "Request URL".into(), required: true, default_value: None, param_type: ParamType::String },
                        ActionParam { name: "headers".into(), description: "JSON object of headers (e.g. {\"Authorization\":\"Bearer x\"})".into(), required: false, default_value: None, param_type: ParamType::Json },
                    ],
                },
                PluginAction {
                    name: "post".into(), description: "Send HTTP POST request with JSON body (use --input for body)".into(),
                    params: vec![
                        ActionParam { name: "url".into(), description: "Request URL".into(), required: true, default_value: None, param_type: ParamType::String },
                        ActionParam { name: "headers".into(), description: "JSON object of headers".into(), required: false, default_value: None, param_type: ParamType::Json },
                    ],
                },
                PluginAction {
                    name: "put".into(), description: "Send HTTP PUT request with JSON body".into(),
                    params: vec![
                        ActionParam { name: "url".into(), description: "Request URL".into(), required: true, default_value: None, param_type: ParamType::String },
                        ActionParam { name: "headers".into(), description: "JSON object of headers".into(), required: false, default_value: None, param_type: ParamType::Json },
                    ],
                },
                PluginAction {
                    name: "delete".into(), description: "Send HTTP DELETE request".into(),
                    params: vec![
                        ActionParam { name: "url".into(), description: "Request URL".into(), required: true, default_value: None, param_type: ParamType::String },
                        ActionParam { name: "headers".into(), description: "JSON object of headers".into(), required: false, default_value: None, param_type: ParamType::Json },
                    ],
                },
                PluginAction {
                    name: "head".into(), description: "Send HTTP HEAD request, show response headers".into(),
                    params: vec![
                        ActionParam { name: "url".into(), description: "Request URL".into(), required: true, default_value: None, param_type: ParamType::String },
                    ],
                },
            ],
        }
    }

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        rt.block_on(self.run_request(&input))
    }

    fn tui_view(&self) -> Option<TuiViewDef> {
        Some(TuiViewDef { title: "HTTP Client".into(), component_type: TuiComponentType::Form })
    }

    fn web_handlers(&self) -> Vec<WebHandlerDef> { vec![] }
}

impl HttpClientPlugin {
    async fn run_request(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let url_str = input.params.get("url")
            .or_else(|| input.input_data.as_ref())
            .ok_or_else(|| PluginError::MissingParam("url".into()))?;

        let headers_json = input.params.get("headers");
        let is_body_method = matches!(input.action.as_str(), "post" | "put");
        let body = if is_body_method { input.input_data.clone() } else { None };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .danger_accept_invalid_certs(false)
            .build()
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let mut req = match input.action.as_str() {
            "get" => client.get(url_str),
            "post" => client.post(url_str),
            "put" => client.put(url_str),
            "delete" => client.delete(url_str),
            "head" => client.head(url_str),
            _ => return Err(PluginError::InvalidAction(input.action.clone())),
        };

        if let Some(hdrs) = headers_json {
            if let Ok(map) = serde_json::from_str::<serde_json::Value>(hdrs) {
                if let Some(obj) = map.as_object() {
                    for (k, v) in obj {
                        let val = match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        req = req.header(k.as_str(), val);
                    }
                }
            }
        }

        if is_body_method {
            req = req.header("Content-Type", "application/json");
        }

        if let Some(ref b) = body {
            req = req.body(b.clone());
        }

        let start = std::time::Instant::now();
        match req.send().await {
            Ok(resp) => {
                let elapsed = start.elapsed();
                let status = resp.status();
                let resp_headers: Vec<String> = resp.headers().iter()
                    .map(|(k, v)| format!("  {}: {}", k, v.to_str().unwrap_or("?")))
                    .collect();
                let resp_body = resp.text().await.unwrap_or_else(|_| "<binary>".into());

                let output = format!(
                    "URL: {}\nStatus: {} {}\nLatency: {:.2}ms\n\nResponse Headers:\n{}\n\nBody:\n{}",
                    url_str, status.as_u16(), status.canonical_reason().unwrap_or(""),
                    elapsed.as_secs_f64() * 1000.0, resp_headers.join("\n"),
                    truncate(&resp_body, 8000),
                );

                Ok(PluginOutput {
                    success: status.is_success(),
                    data: output,
                    error: if status.is_success() { None } else { Some(format!("HTTP {}", status.as_u16())) },
                    metadata: Some([
                        ("status".into(), status.as_u16().to_string()),
                        ("latency_ms".into(), format!("{:.2}", elapsed.as_secs_f64() * 1000.0)),
                    ].into_iter().collect()),
                })
            }
            Err(e) => {
                let elapsed = start.elapsed();
                let mut msg = format!("Request failed: {}\nLatency: {:.2}ms", e, elapsed.as_secs_f64() * 1000.0);
                if e.is_timeout() { msg.push_str("\nHint: Request timed out."); }
                if e.is_connect() { msg.push_str("\nHint: Connection refused."); }
                Ok(PluginOutput { success: false, data: msg, error: Some(e.to_string()), metadata: None })
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.into() }
    else { format!("{}...\n[truncated, {} total chars]", &s[..max], s.len()) }
}

#[no_mangle]
pub extern "C" fn _plugin_create() -> Box<dyn Plugin> {
    Box::new(HttpClientPlugin)
}
