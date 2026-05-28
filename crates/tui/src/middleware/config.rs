use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let mut p = PathBuf::from(home);
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

pub fn save_config(cfg: &MiddlewareConfig) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_roundtrip() {
        let cfg = MiddlewareConfig {
            redis: vec![RedisConn {
                name: "test-redis".into(), host: "127.0.0.1".into(),
                port: 6379, password: Some("secret".into()), db: 0,
            }],
            ..Default::default()
        };
        let s = serde_json::to_string(&cfg).unwrap();
        let back: MiddlewareConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(back.redis.len(), 1);
        assert_eq!(back.redis[0].name, "test-redis");
    }

    #[test]
    fn test_add_remove() {
        let conn = RedisConn { name: "t".into(), host: "h".into(), port: 6379, password: None, db: 0 };
        add_redis_conn(conn);
        let cfg = load_config();
        assert!(cfg.redis.iter().any(|c| c.name == "t"));
        remove_redis_conn("t");
        let cfg2 = load_config();
        assert!(!cfg2.redis.iter().any(|c| c.name == "t"));
    }
}
