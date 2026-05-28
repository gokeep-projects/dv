use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;
use std::net::{TcpStream, ToSocketAddrs};
use std::process::Command;
use std::time::Duration;

struct ServiceStatusPlugin;

impl Plugin for ServiceStatusPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "service-status".into(),
            version: "0.1.0".into(),
            description: "Monitor service health, check ports, and verify process status".into(),
            author: "DevTool Team".into(),
            category: PluginCategory::Network,
            actions: vec![
                PluginAction {
                    name: "http-check".into(),
                    description: "Check if an HTTP/HTTPS endpoint is reachable and measure latency".into(),
                    params: vec![
                        ActionParam {
                            name: "url".into(),
                            description: "URL to check (e.g. http://localhost:8080/health)".into(),
                            required: true,
                            default_value: None,
                            param_type: ParamType::String,
                        },
                        ActionParam {
                            name: "timeout".into(),
                            description: "Timeout in seconds (default: 5)".into(),
                            required: false,
                            default_value: Some("5".into()),
                            param_type: ParamType::Number,
                        },
                    ],
                },
                PluginAction {
                    name: "tcp-check".into(),
                    description: "Check if a TCP port is open and accepting connections".into(),
                    params: vec![
                        ActionParam {
                            name: "host".into(),
                            description: "Host to connect to".into(),
                            required: true,
                            default_value: Some("localhost".into()),
                            param_type: ParamType::String,
                        },
                        ActionParam {
                            name: "port".into(),
                            description: "TCP port number".into(),
                            required: true,
                            default_value: None,
                            param_type: ParamType::Number,
                        },
                    ],
                },
                PluginAction {
                    name: "process-check".into(),
                    description: "Check if a process is running by name or PID".into(),
                    params: vec![ActionParam {
                        name: "name".into(),
                        description: "Process name to search for".into(),
                        required: true,
                        default_value: None,
                        param_type: ParamType::String,
                    }],
                },
                PluginAction {
                    name: "dns-lookup".into(),
                    description: "Resolve a hostname to IP addresses".into(),
                    params: vec![ActionParam {
                        name: "hostname".into(),
                        description: "Hostname to resolve".into(),
                        required: true,
                        default_value: None,
                        param_type: ParamType::String,
                    }],
                },
            ],
        }
    }

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        match input.action.as_str() {
            "http-check" => self.http_check(&input),
            "tcp-check" => self.tcp_check(&input),
            "process-check" => self.process_check(&input),
            "dns-lookup" => self.dns_lookup(&input),
            _ => Err(PluginError::InvalidAction(input.action)),
        }
    }

    fn tui_view(&self) -> Option<TuiViewDef> {
        Some(TuiViewDef {
            title: "Service Status".into(),
            component_type: TuiComponentType::Table,
        })
    }

    fn web_handlers(&self) -> Vec<WebHandlerDef> {
        vec![]
    }
}

impl ServiceStatusPlugin {
    fn http_check(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let url = input
            .params
            .get("url")
            .ok_or_else(|| PluginError::MissingParam("url".into()))?;

        let timeout: u64 = input
            .params
            .get("timeout")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        let start = std::time::Instant::now();

        // Use curl or a minimal HTTP GET via TCP
        let client = std::net::TcpStream::connect_timeout(
            &parse_host_port(url)?,
            Duration::from_secs(timeout),
        );

        let elapsed = start.elapsed();

        match client {
            Ok(stream) => {
                // Send minimal HTTP request
                let mut s = stream;
                s.set_read_timeout(Some(Duration::from_secs(timeout))).ok();
                use std::io::Write;
                let host = extract_host(url);
                let request = format!(
                    "GET {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: DevTool/1.0\r\nConnection: close\r\n\r\n",
                    extract_path(url),
                    host
                );
                let _ = s.write_all(request.as_bytes());

                use std::io::Read;
                let mut response = vec![0u8; 512];
                let _ = s.read(&mut response);
                let head = String::from_utf8_lossy(&response);
                let status_line = head.lines().next().unwrap_or("No response");

                let is_success = status_line.contains("200")
                    || status_line.contains("301")
                    || status_line.contains("302");

                Ok(PluginOutput {
                    success: true,
                    data: format!(
                        "URL: {}\nStatus: {}\nLatency: {:.2}ms\nResult: {}",
                        url,
                        status_line,
                        elapsed.as_secs_f64() * 1000.0,
                        if is_success { "✓ OK" } else { "⚠ Non-200" },
                    ),
                    error: if is_success { None } else { Some("Non-success status".into()) },
                    metadata: Some(
                        [
                            ("latency_ms".into(), format!("{:.2}", elapsed.as_secs_f64() * 1000.0)),
                            ("status".into(), status_line.to_string()),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                })
            }
            Err(e) => Ok(PluginOutput {
                success: false,
                data: format!("✗ Failed to connect to {}\nError: {}\nLatency: {:.2}ms", url, e, elapsed.as_secs_f64() * 1000.0),
                error: Some(e.to_string()),
                metadata: None,
            }),
        }
    }

    fn tcp_check(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let host = input
            .params
            .get("host")
            .map(|s| s.as_str())
            .unwrap_or("localhost");
        let port: u16 = input
            .params
            .get("port")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| PluginError::MissingParam("port".into()))?;

        let start = std::time::Instant::now();
        let addr = (host, port)
            .to_socket_addrs()
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?
            .next()
            .ok_or_else(|| PluginError::ExecutionFailed("Cannot resolve address".into()))?;

        match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
            Ok(_) => {
                let elapsed = start.elapsed();
                Ok(PluginOutput {
                    success: true,
                    data: format!("✓ TCP {}:{} is OPEN ({}ms)", host, port, elapsed.as_millis()),
                    error: None,
                    metadata: Some(
                        [("latency_ms".into(), elapsed.as_millis().to_string())]
                            .into_iter()
                            .collect(),
                    ),
                })
            }
            Err(e) => Ok(PluginOutput {
                success: false,
                data: format!("✗ TCP {}:{} is CLOSED or unreachable: {}", host, port, e),
                error: Some(e.to_string()),
                metadata: None,
            }),
        }
    }

    fn process_check(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let name = input
            .params
            .get("name")
            .ok_or_else(|| PluginError::MissingParam("name".into()))?;

        let output = if cfg!(target_os = "windows") {
            Command::new("tasklist")
                .args(["/FI", &format!("IMAGENAME eq {}", name)])
                .output()
        } else {
            Command::new("pgrep").arg("-a").arg(name).output()
        };

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}{}", stdout, stderr).trim().to_string();

                if combined.is_empty() && !out.status.success() {
                    Ok(PluginOutput {
                        success: true,
                        data: format!("✗ Process '{}' is NOT running", name),
                        error: None,
                        metadata: Some(
                            [("running".into(), "false".into())].into_iter().collect(),
                        ),
                    })
                } else {
                    Ok(PluginOutput {
                        success: true,
                        data: format!("✓ Process '{}' is running\n\nDetails:\n{}", name, combined),
                        error: None,
                        metadata: Some(
                            [("running".into(), "true".into()), ("details".into(), combined)]
                                .into_iter()
                                .collect(),
                        ),
                    })
                }
            }
            Err(e) => {
                // Try with ps as fallback
                let ps_out = Command::new("ps").args(["-eo", "pid,comm"]).output();
                match ps_out {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let matches: Vec<&str> = stdout
                            .lines()
                            .filter(|l| l.contains(name))
                            .collect();
                        if matches.is_empty() {
                            Ok(PluginOutput {
                                success: true,
                                data: format!("✗ Process '{}' is NOT running", name),
                                error: None,
                                metadata: None,
                            })
                        } else {
                            Ok(PluginOutput {
                                success: true,
                                data: format!(
                                    "✓ Found {} process(es) matching '{}':\n{}",
                                    matches.len(),
                                    name,
                                    matches.join("\n")
                                ),
                                error: None,
                                metadata: None,
                            })
                        }
                    }
                    Err(_) => Ok(PluginOutput {
                        success: false,
                        data: format!("Failed to check process: {}", e),
                        error: Some(e.to_string()),
                        metadata: None,
                    }),
                }
            }
        }
    }

    fn dns_lookup(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let hostname = input
            .params
            .get("hostname")
            .ok_or_else(|| PluginError::MissingParam("hostname".into()))?;

        let start = std::time::Instant::now();
        match format!("{}:80", hostname).to_socket_addrs() {
            Ok(addrs) => {
                let elapsed = start.elapsed();
                let mut result = format!(
                    "DNS Lookup: {}\nResolved in: {:.2}ms\nAddresses:\n",
                    hostname,
                    elapsed.as_secs_f64() * 1000.0
                );
                for addr in addrs {
                    result.push_str(&format!("  - {}\n", addr.ip()));
                }
                Ok(PluginOutput {
                    success: true,
                    data: result,
                    error: None,
                    metadata: None,
                })
            }
            Err(e) => Ok(PluginOutput {
                success: false,
                data: format!("✗ DNS lookup failed for '{}': {}", hostname, e),
                error: Some(e.to_string()),
                metadata: None,
            }),
        }
    }
}

fn parse_host_port(url: &str) -> PluginResult<std::net::SocketAddr> {
    let host = extract_host(url);
    let port = if url.starts_with("https://") {
        443u16
    } else {
        80u16
    };
    // Check if URL specifies a port
    let addr_str = if let Some(after_host) = url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .strip_prefix(host) {
        if after_host.starts_with(':') {
            let port_part: String = after_host.chars().skip(1).take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(p) = port_part.parse::<u16>() {
                format!("{}:{}", host, p)
            } else {
                format!("{}:{}", host, port)
            }
        } else {
            format!("{}:{}", host, port)
        }
    } else {
        format!("{}:{}", host, port)
    };

    addr_str
        .parse()
        .map_err(|e| PluginError::ExecutionFailed(format!("Cannot resolve address: {}", e)))
}

fn extract_host(url: &str) -> &str {
    let url = url.trim_start_matches("https://").trim_start_matches("http://");
    url.split('/').next().unwrap_or(url)
        .split(':').next().unwrap_or("localhost")
}

fn extract_path(url: &str) -> &str {
    let after_scheme = url.trim_start_matches("https://").trim_start_matches("http://");
    if let Some(pos) = after_scheme.find('/') {
        &after_scheme[pos..]
    } else {
        "/"
    }
}

#[no_mangle]
pub extern "C" fn _plugin_create() -> Box<dyn Plugin> {
    Box::new(ServiceStatusPlugin)
}
