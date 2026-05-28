use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

struct MiddlewarePlugin;

impl Plugin for MiddlewarePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "middleware".into(),
            version: "0.1.0".into(),
            description: "Manage and test connections to middleware services (Redis, MySQL, Kafka, Elasticsearch)".into(),
            author: "DevTool Team".into(),
            category: PluginCategory::Middleware,
            actions: vec![
                PluginAction {
                    name: "redis-ping".into(),
                    description: "Test connection to a Redis server".into(),
                    params: vec![
                        ActionParam {
                            name: "host".into(),
                            description: "Redis host".into(),
                            required: false,
                            default_value: Some("localhost".into()),
                            param_type: ParamType::String,
                        },
                        ActionParam {
                            name: "port".into(),
                            description: "Redis port (default: 6379)".into(),
                            required: false,
                            default_value: Some("6379".into()),
                            param_type: ParamType::Number,
                        },
                        ActionParam {
                            name: "password".into(),
                            description: "Redis password".into(),
                            required: false,
                            default_value: None,
                            param_type: ParamType::String,
                        },
                    ],
                },
                PluginAction {
                    name: "mysql-ping".into(),
                    description: "Test connection to a MySQL/MariaDB server".into(),
                    params: vec![
                        ActionParam {
                            name: "host".into(),
                            description: "MySQL host".into(),
                            required: false,
                            default_value: Some("localhost".into()),
                            param_type: ParamType::String,
                        },
                        ActionParam {
                            name: "port".into(),
                            description: "MySQL port (default: 3306)".into(),
                            required: false,
                            default_value: Some("3306".into()),
                            param_type: ParamType::Number,
                        },
                    ],
                },
                PluginAction {
                    name: "kafka-brokers".into(),
                    description: "Test connection to Kafka brokers".into(),
                    params: vec![ActionParam {
                        name: "brokers".into(),
                        description: "Comma-separated broker addresses (e.g. localhost:9092)".into(),
                        required: true,
                        default_value: Some("localhost:9092".into()),
                        param_type: ParamType::String,
                    }],
                },
                PluginAction {
                    name: "elasticsearch".into(),
                    description: "Test connection to Elasticsearch cluster".into(),
                    params: vec![
                        ActionParam {
                            name: "url".into(),
                            description: "Elasticsearch URL".into(),
                            required: false,
                            default_value: Some("http://localhost:9200".into()),
                            param_type: ParamType::String,
                        },
                    ],
                },
                PluginAction {
                    name: "port-scan".into(),
                    description: "Scan common ports on a host".into(),
                    params: vec![
                        ActionParam {
                            name: "host".into(),
                            description: "Host to scan".into(),
                            required: true,
                            default_value: Some("localhost".into()),
                            param_type: ParamType::String,
                        },
                        ActionParam {
                            name: "ports".into(),
                            description: "Comma-separated port list or range (e.g. 80,443 or 8000-8100)".into(),
                            required: false,
                            default_value: Some("common".into()),
                            param_type: ParamType::String,
                        },
                    ],
                },
            ],
        }
    }

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        match input.action.as_str() {
            "redis-ping" => self.redis_ping(&input),
            "mysql-ping" => self.mysql_ping(&input),
            "kafka-brokers" => self.kafka_check(&input),
            "elasticsearch" => self.es_check(&input),
            "port-scan" => self.port_scan(&input),
            _ => Err(PluginError::InvalidAction(input.action)),
        }
    }

    fn tui_view(&self) -> Option<TuiViewDef> {
        Some(TuiViewDef {
            title: "Middleware Manager".into(),
            component_type: TuiComponentType::Table,
        })
    }

    fn web_handlers(&self) -> Vec<WebHandlerDef> {
        vec![]
    }
}

impl MiddlewarePlugin {
    fn tcp_test(host: &str, port: u16, name: &str) -> (bool, String) {
        let addr_str = format!("{}:{}", host, port);
        let start = std::time::Instant::now();
        let addr = match (host, port).to_socket_addrs().ok().and_then(|mut a| a.next()) {
            Some(a) => a,
            None => return (false, format!("✗ {} ({}) — cannot resolve", name, addr_str)),
        };
        match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
            Ok(_) => {
                let elapsed = start.elapsed();
                (true, format!("✓ {} ({}) — {}ms", name, addr_str, elapsed.as_millis()))
            }
            Err(e) => (false, format!("✗ {} ({}) — {}", name, addr_str, e)),
        }
    }

    fn redis_ping(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let host = input.params.get("host").map(|s| s.as_str()).unwrap_or("localhost");
        let port: u16 = input.params.get("port").and_then(|s| s.parse().ok()).unwrap_or(6379);

        let addr_str = format!("{}:{}", host, port);
        let addr = match (host, port).to_socket_addrs().ok().and_then(|mut a| a.next()) {
            Some(a) => a,
            None => return Ok(PluginOutput {
                success: false,
                data: format!("✗ Cannot resolve {}:{}", host, port),
                error: Some("DNS resolution failed".into()),
                metadata: None,
            }),
        };
        match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
            Ok(mut stream) => {
                use std::io::{Read, Write};
                let cmd = if let Some(pw) = input.params.get("password") {
                    format!("AUTH {}\r\nPING\r\n", pw)
                } else {
                    "PING\r\n".to_string()
                };
                let _ = stream.write_all(cmd.as_bytes());
                stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
                let mut buf = [0u8; 128];
                let _ = stream.read(&mut buf);
                let response = String::from_utf8_lossy(&buf).trim().to_string();
                let ok = response.contains("PONG");
                Ok(PluginOutput {
                    success: ok,
                    data: format!("Redis {}\nResponse: {}", addr_str, response),
                    error: if ok { None } else { Some("No PONG response".into()) },
                    metadata: None,
                })
            }
            Err(e) => Ok(PluginOutput {
                success: false,
                data: format!("✗ Cannot connect to Redis at {}: {}", addr_str, e),
                error: Some(e.to_string()),
                metadata: None,
            }),
        }
    }

    fn mysql_ping(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let host = input.params.get("host").map(|s| s.as_str()).unwrap_or("localhost");
        let port: u16 = input.params.get("port").and_then(|s| s.parse().ok()).unwrap_or(3306);
        let (ok, msg) = Self::tcp_test(host, port, "MySQL");
        Ok(PluginOutput {
            success: ok,
            data: msg,
            error: if ok { None } else { Some("Connection refused".into()) },
            metadata: None,
        })
    }

    fn kafka_check(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let brokers = input.params.get("brokers").map(|s| s.as_str()).unwrap_or("localhost:9092");
        let mut result = Vec::new();
        let mut all_ok = true;

        for broker in brokers.split(',') {
            let broker = broker.trim();
            let parts: Vec<&str> = broker.split(':').collect();
            let host = parts.first().copied().unwrap_or("localhost");
            let port: u16 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(9092);

            let (ok, msg) = Self::tcp_test(host, port, &format!("Kafka broker {}", broker));
            if !ok {
                all_ok = false;
            }
            result.push(msg);
        }

        Ok(PluginOutput {
            success: all_ok,
            data: result.join("\n"),
            error: if all_ok { None } else { Some("Some brokers unreachable".into()) },
            metadata: None,
        })
    }

    fn es_check(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let url = input.params.get("url").map(|s| s.as_str()).unwrap_or("http://localhost:9200");
        let host = url.trim_start_matches("https://").trim_start_matches("http://").split('/').next().unwrap_or("localhost:9200");
        let parts: Vec<&str> = host.split(':').collect();
        let hostname = parts.first().copied().unwrap_or("localhost");
        let port: u16 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(9200);

        let (ok, msg) = Self::tcp_test(hostname, port, "Elasticsearch");
        Ok(PluginOutput {
            success: ok,
            data: format!("{}\nURL: {}", msg, url),
            error: if ok { None } else { Some("Connection refused".into()) },
            metadata: None,
        })
    }

    fn port_scan(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let host = input.params.get("host").map(|s| s.as_str()).unwrap_or("localhost");
        let ports_str = input.params.get("ports").map(|s| s.as_str()).unwrap_or("common");
        let ports = parse_ports(ports_str);
        let mut result = Vec::new();
        result.push(format!("Port Scan: {}\n{}", host, "═".repeat(50)));

        let mut open_count = 0;
        for port in &ports {
            let (ok, _msg) = Self::tcp_test(host, *port, "");
            if ok {
                open_count += 1;
                result.push(format!("  {:>6}  ✓ OPEN    {}", port, service_name(*port)));
            }
        }

        result.push(format!("\nScanned {} ports, {} open on {}", ports.len(), open_count, host));

        Ok(PluginOutput {
            success: true,
            data: result.join("\n"),
            error: None,
            metadata: Some([
                ("open".into(), open_count.to_string()),
                ("total".into(), ports.len().to_string()),
            ].into_iter().collect()),
        })
    }
}

fn parse_ports(ports_str: &str) -> Vec<u16> {
    if ports_str == "common" {
        return vec![21, 22, 23, 25, 53, 80, 110, 143, 443, 993, 995,
            3306, 3389, 5432, 6379, 8080, 8443, 9092, 9200, 27017];
    }
    let mut ports = Vec::new();
    for part in ports_str.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let range: Vec<&str> = part.split('-').collect();
            if let (Some(&start), Some(&end)) = (range.first(), range.get(1)) {
                if let (Ok(s), Ok(e)) = (start.parse::<u16>(), end.parse::<u16>()) {
                    for p in s..=e {
                        ports.push(p);
                    }
                }
            }
        } else if let Ok(p) = part.parse::<u16>() {
            ports.push(p);
        }
    }
    ports
}

fn service_name(port: u16) -> &'static str {
    match port {
        21 => "FTP",
        22 => "SSH",
        25 => "SMTP",
        53 => "DNS",
        80 => "HTTP",
        110 => "POP3",
        143 => "IMAP",
        443 => "HTTPS",
        993 => "IMAPS",
        995 => "POP3S",
        3306 => "MySQL",
        3389 => "RDP",
        5432 => "PostgreSQL",
        6379 => "Redis",
        8080 => "HTTP-Alt",
        8443 => "HTTPS-Alt",
        9092 => "Kafka",
        9200 => "Elasticsearch",
        27017 => "MongoDB",
        _ => "",
    }
}

#[no_mangle]
pub extern "C" fn _plugin_create() -> Box<dyn Plugin> {
    Box::new(MiddlewarePlugin)
}
