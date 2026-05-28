use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MiddlewareConfig {
    #[serde(default)]
    pub redis: Vec<RedisConn>,
    #[serde(default)]
    pub elasticsearch: Vec<EsConn>,
    #[serde(default)]
    pub kafka: Vec<KafkaConn>,
    #[serde(default)]
    pub nginx: Vec<NginxConn>,
    #[serde(default)]
    pub tomcat: Vec<TomcatConn>,
    #[serde(default)]
    pub caddy: Vec<CaddyConn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConn {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub db: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsConn {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    pub password: Option<String>,
    pub scheme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConn {
    pub name: String,
    pub brokers: String,
    pub sasl_user: Option<String>,
    pub sasl_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NginxConn {
    pub name: String,
    pub config_path: Option<String>,
    pub log_path: Option<String>,
    pub pid_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomcatConn {
    pub name: String,
    pub catalina_home: Option<String>,
    pub log_path: Option<String>,
    pub pid_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaddyConn {
    pub name: String,
    pub config_path: Option<String>,
    pub log_path: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct DiscoveredService {
    pub name: String,
    pub mw_type: String,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub config_path: Option<String>,
    pub log_path: Option<String>,
    pub version: Option<String>,
    pub status: String,
}

fn config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let mut p = std::path::PathBuf::from(home);
    p.push(".devtool");
    std::fs::create_dir_all(&p).ok();
    p.push("middleware.json");
    p
}

pub fn load_config() -> MiddlewareConfig {
    let path = config_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        MiddlewareConfig::default()
    }
}

fn save_config(cfg: &MiddlewareConfig) {
    let path = config_path();
    if let Ok(s) = serde_json::to_string_pretty(cfg) {
        std::fs::write(&path, s).ok();
    }
}

macro_rules! add_remove_impl {
    ($add:ident, $remove:ident, $field:ident, $typ:ty) => {
        pub fn $add(conn: $typ) {
            let mut cfg = load_config();
            cfg.$field.retain(|c| c.name != conn.name);
            cfg.$field.push(conn);
            save_config(&cfg);
        }
        pub fn $remove(name: &str) {
            let mut cfg = load_config();
            cfg.$field.retain(|c| c.name != name);
            save_config(&cfg);
        }
    };
}

add_remove_impl!(add_redis_conn, remove_redis_conn, redis, RedisConn);
add_remove_impl!(add_es_conn, remove_es_conn, elasticsearch, EsConn);
add_remove_impl!(add_kafka_conn, remove_kafka_conn, kafka, KafkaConn);
add_remove_impl!(add_nginx_conn, remove_nginx_conn, nginx, NginxConn);
add_remove_impl!(add_tomcat_conn, remove_tomcat_conn, tomcat, TomcatConn);
add_remove_impl!(add_caddy_conn, remove_caddy_conn, caddy, CaddyConn);

pub fn redis_cli(host: &str, port: u16, password: &Option<String>, cmd: &str) -> Result<String, String> {
    let port_s = port.to_string();
    let mut args = vec!["-h", host, "-p", &port_s];
    let pass_str;
    if let Some(p) = password {
        if !p.is_empty() {
            pass_str = p.clone();
            args.push("-a");
            args.push(&pass_str);
            args.push("--no-auth-warning");
        }
    }
    let cmd_args: Vec<&str> = cmd.split_whitespace().collect();
    args.extend_from_slice(&cmd_args);
    match Command::new("redis-cli").args(&args).output() {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            let e = String::from_utf8_lossy(&o.stderr).to_string();
            if o.status.success() { Ok(if s.is_empty() { e } else { s }) }
            else { Err(if e.is_empty() { s } else { e }) }
        }
        Err(e) => Err(format!("redis-cli not found: {}", e)),
    }
}

pub fn discover_services() -> Vec<DiscoveredService> {
    let mut services = Vec::new();
    let sigs: Vec<(&str, &[u16], &str)> = vec![
        ("redis-server", &[6379], "redis"),
        ("mysqld|mariadbd", &[3306], "mysql"),
        ("postgres", &[5432], "postgresql"),
        ("mongod", &[27017], "mongodb"),
        ("nginx", &[80, 443, 8080], "nginx"),
        ("tomcat|catalina", &[8080], "tomcat"),
        ("caddy", &[80, 443, 2015], "caddy"),
        ("kafka", &[9092], "kafka"),
        ("elasticsearch", &[9200], "elasticsearch"),
        ("rabbitmq", &[5672], "rabbitmq"),
        ("dockerd", &[2375, 2376], "docker"),
    ];
    let all_ports = get_listening_ports();
    for (proc_pat, ports, mw_type) in &sigs {
        if let Ok(out) = Command::new("sh").arg("-c")
            .arg(&format!("ps aux 2>/dev/null | grep -E '{}' | grep -v grep", proc_pat)).output()
        {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                if line.is_empty() { continue; }
                let parts: Vec<&str> = line.split_whitespace().collect();
                let pid = parts.get(1).and_then(|p| p.parse::<u32>().ok());
                let mut svc = DiscoveredService {
                    name: mw_type.to_string(), mw_type: mw_type.to_string(),
                    pid, port: None, config_path: None, log_path: None,
                    version: None, status: "Running".to_string(),
                };
                for p in *ports { if all_ports.contains(p) { svc.port = Some(*p); break; } }
                svc.version = detect_version(&svc.mw_type);
                services.push(svc);
            }
        }
        if !services.iter().any(|s| s.mw_type == *mw_type) && !ports.is_empty() {
            for p in *ports {
                if all_ports.contains(p) {
                    services.push(DiscoveredService {
                        name: mw_type.to_string(), mw_type: mw_type.to_string(),
                        pid: None, port: Some(*p), config_path: None, log_path: None,
                        version: None, status: "Running".to_string(),
                    });
                    break;
                }
            }
        }
    }
    services
}

fn get_listening_ports() -> Vec<u16> {
    let mut ports = Vec::new();
    if let Ok(out) = Command::new("sh").arg("-c")
        .arg("ss -tlnp 2>/dev/null | awk '{print $4}' | grep -oP ':\\K\\d+'").output()
    {
        for p in String::from_utf8_lossy(&out.stdout).lines() {
            if let Ok(n) = p.trim().parse() { ports.push(n); }
        }
    }
    ports
}

fn detect_version(mw_type: &str) -> Option<String> {
    let (cmd, args): (&str, Vec<&str>) = match mw_type {
        "redis" => ("redis-cli", vec!["--version"]),
        "mysql" => ("mysql", vec!["--version"]),
        "postgresql" => ("psql", vec!["--version"]),
        "nginx" => ("nginx", vec!["-v"]),
        "docker" => ("docker", vec!["--version"]),
        "elasticsearch" => ("curl", vec!["-s", "localhost:9200"]),
        _ => return None,
    };
    if let Ok(out) = Command::new(cmd).args(&args).output() {
        let s = String::from_utf8_lossy(&out.stdout).to_string();
        let e = String::from_utf8_lossy(&out.stderr).to_string();
        let combined = if s.is_empty() { e } else { s };
        let v = combined.lines().next().unwrap_or("").trim().to_string();
        if !v.is_empty() { Some(v) } else { None }
    } else { None }
}
