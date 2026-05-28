use std::process::Command;

#[derive(Debug, Clone)]
pub struct DiscoveredService {
    pub name: String,
    pub mw_type: String,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub config_path: Option<String>,
    pub log_path: Option<String>,
    pub version: Option<String>,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus { Running, Stopped, Unknown }

pub fn discover_all() -> Vec<DiscoveredService> {
    let mut services = Vec::new();
    let sigs: Vec<(&str, &[u16], &str, &[&str], &[&str])> = vec![
        ("redis-server", &[6379], "redis", &["/etc/redis/redis.conf"], &["/var/log/redis/"]),
        ("mysqld|mariadbd", &[3306], "mysql", &["/etc/mysql/my.cnf","/etc/my.cnf"], &["/var/log/mysql/"]),
        ("postgres", &[5432], "postgresql", &["/etc/postgresql"], &["/var/log/postgresql/"]),
        ("mongod", &[27017,27018], "mongodb", &["/etc/mongod.conf"], &["/var/log/mongodb/"]),
        ("nginx", &[80,443,8080], "nginx", &["/etc/nginx/nginx.conf"], &["/var/log/nginx/"]),
        ("httpd|apache2", &[80,443,8080], "apache", &["/etc/httpd/conf/httpd.conf","/etc/apache2/apache2.conf"], &["/var/log/httpd/"]),
        ("tomcat|catalina", &[8080,8443,8009], "tomcat", &["/opt/tomcat/conf/server.xml"], &["/opt/tomcat/logs/"]),
        ("caddy", &[80,443,2015,2019], "caddy", &["/etc/caddy/Caddyfile"], &["/var/log/caddy/"]),
        ("kafka\\.Kafka|kafka-server", &[9092,9093], "kafka", &["/etc/kafka/server.properties","/opt/kafka/config/server.properties"], &["/var/log/kafka/"]),
        ("elasticsearch", &[9200,9300], "elasticsearch", &["/etc/elasticsearch/elasticsearch.yml"], &["/var/log/elasticsearch/"]),
        ("rabbitmq|beam\\.smp.*rabbit", &[5672,15672,25672], "rabbitmq", &["/etc/rabbitmq/rabbitmq.conf"], &["/var/log/rabbitmq/"]),
        ("haproxy", &[80,443,8080,8404], "haproxy", &["/etc/haproxy/haproxy.cfg"], &["/var/log/haproxy/"]),
        ("keepalived", &[], "keepalived", &["/etc/keepalived/keepalived.conf"], &["/var/log/keepalived/"]),
        ("etcd", &[2379,2380], "etcd", &["/etc/etcd/etcd.conf"], &["/var/log/etcd/"]),
        ("consul", &[8500,8300,8301], "consul", &["/etc/consul.d/"], &["/var/log/consul/"]),
        ("zookeeper|QuorumPeerMain", &[2181,2888,3888], "zookeeper", &["/etc/zookeeper/conf/zoo.cfg"], &["/var/log/zookeeper/"]),
        ("minio", &[9000,9001], "minio", &["/etc/minio/config.json"], &["/var/log/minio/"]),
        ("prometheus", &[9090], "prometheus", &["/etc/prometheus/prometheus.yml"], &["/var/log/prometheus/"]),
        ("grafana", &[3000], "grafana", &["/etc/grafana/grafana.ini"], &["/var/log/grafana/"]),
        ("memcached", &[11211], "memcached", &["/etc/memcached.conf"], &["/var/log/memcached/"]),
        ("php-fpm|php.*-fpm", &[9000,9001], "php-fpm", &["/etc/php"], &["/var/log/"]),
        ("dockerd", &[2375,2376], "docker", &["/etc/docker/daemon.json"], &["/var/log/"]),
        ("sshd", &[22], "sshd", &["/etc/ssh/sshd_config"], &["/var/log/auth.log"]),
        ("named|bind", &[53], "bind", &["/etc/bind/named.conf"], &["/var/log/"]),
        ("supervisord", &[9001], "supervisor", &["/etc/supervisor/supervisord.conf"], &["/var/log/supervisor/"]),
        ("jenkins", &[8080,50000], "jenkins", &["/etc/default/jenkins"], &["/var/log/jenkins/"]),
    ];

    // Get all listening ports
    let all_ports = get_listening_ports();

    for (proc_pat, ports, mw_type, configs, log_dirs) in &sigs {
        let mut found = false;

        // Check processes
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
                    version: None, status: ServiceStatus::Running,
                };
                // Match port
                for p in *ports {
                    if all_ports.contains(p) { svc.port = Some(*p); break; }
                }
                // Config
                for cfg in *configs {
                    if std::path::Path::new(cfg).exists() { svc.config_path = Some(cfg.to_string()); break; }
                }
                // Log
                for ld in *log_dirs {
                    if std::path::Path::new(ld).exists() { svc.log_path = Some(ld.to_string()); break; }
                }
                svc.version = detect_version(&svc);
                services.push(svc);
                found = true;
            }
        }

        // Check ports alone
        if !found && !ports.is_empty() {
            for p in *ports {
                if all_ports.contains(p) {
                    services.push(DiscoveredService {
                        name: mw_type.to_string(), mw_type: mw_type.to_string(),
                        pid: None, port: Some(*p),
                        config_path: configs.iter().find(|c| std::path::Path::new(c).exists()).map(|c| c.to_string()),
                        log_path: log_dirs.iter().find(|l| std::path::Path::new(l).exists()).map(|l| l.to_string()),
                        version: None, status: ServiceStatus::Running,
                    });
                    break;
                }
            }
        }

        // Check config files
        if !found {
            for cfg in *configs {
                if std::path::Path::new(cfg).exists() {
                    services.push(DiscoveredService {
                        name: mw_type.to_string(), mw_type: mw_type.to_string(),
                        pid: None, port: None, config_path: Some(cfg.to_string()),
                        log_path: log_dirs.iter().find(|l| std::path::Path::new(l).exists()).map(|l| l.to_string()),
                        version: None, status: ServiceStatus::Stopped,
                    });
                    break;
                }
            }
        }
    }

    services.sort_by(|a, b| {
        let ord = |s: &ServiceStatus| match s { ServiceStatus::Running => 0, ServiceStatus::Unknown => 1, ServiceStatus::Stopped => 2 };
        ord(&a.status).cmp(&ord(&b.status)).then(a.name.cmp(&b.name))
    });
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

fn detect_version(svc: &DiscoveredService) -> Option<String> {
    let (cmd, args): (&str, Vec<&str>) = match svc.mw_type.as_str() {
        "redis" => ("redis-cli", vec!["--version"]),
        "mysql" => ("mysql", vec!["--version"]),
        "postgresql" => ("psql", vec!["--version"]),
        "nginx" => ("nginx", vec!["-v"]),
        "mongodb" => ("mongosh", vec!["--version"]),
        "rabbitmq" => ("rabbitmqctl", vec!["version"]),
        "haproxy" => ("haproxy", vec!["-v"]),
        "etcd" => ("etcd", vec!["--version"]),
        "caddy" => ("caddy", vec!["version"]),
        "prometheus" => ("prometheus", vec!["--version"]),
        "grafana" => ("grafana-server", vec!["-v"]),
        "docker" => ("docker", vec!["--version"]),
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
