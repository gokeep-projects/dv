use serde::Serialize;
use std::process::Command;

#[derive(Serialize, Clone)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: String,
    pub cpu: String,
    pub mem: String,
}

#[derive(Serialize, Clone)]
pub struct Image {
    pub repo: String,
    pub tag: String,
    pub id: String,
    pub size: String,
}

#[derive(Serialize, Clone)]
pub struct ComposeProject {
    pub name: String,
    pub path: String,
    pub services: Vec<ComposeService>,
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct ComposeService {
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: String,
}

fn docker(args: &[&str]) -> (String, bool) {
    let mut cmd = Command::new("docker");
    for a in args { cmd.arg(a); }
    match cmd.output() {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            let e = String::from_utf8_lossy(&o.stderr).to_string();
            (if s.is_empty() { e } else { s }, !o.status.success())
        }
        Err(e) => (e.to_string(), true),
    }
}

fn compose(args: &[&str]) -> (String, bool) {
    // Try docker compose (v2) first, then docker-compose (v1)
    let mut cmd = Command::new("docker");
    cmd.arg("compose");
    for a in args { cmd.arg(a); }
    match cmd.output() {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            let e = String::from_utf8_lossy(&o.stderr).to_string();
            if !o.status.success() && e.contains("unknown command") {
                // Fallback to docker-compose
                let mut cmd2 = Command::new("docker-compose");
                for a in args { cmd2.arg(a); }
                match cmd2.output() {
                    Ok(o2) => {
                        let s2 = String::from_utf8_lossy(&o2.stdout).to_string();
                        let e2 = String::from_utf8_lossy(&o2.stderr).to_string();
                        (if s2.is_empty() { e2 } else { s2 }, !o2.status.success())
                    }
                    Err(e2) => (e2.to_string(), true),
                }
            } else {
                (if s.is_empty() { e } else { s }, !o.status.success())
            }
        }
        Err(e) => (e.to_string(), true),
    }
}

pub fn list_containers() -> Vec<Container> {
    let (out, _) = docker(&["ps", "-a", "--format", "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}"]);
    let (stats_out, _) = docker(&["stats", "--no-stream", "--format", "{{.Name}}\t{{.CPUPerc}}\t{{.MemPerc}}"]);
    let stats_map: std::collections::HashMap<String,(String,String)> = stats_out.lines()
        .filter_map(|l| { let p: Vec<&str> = l.split('\t').collect(); if p.len() >= 3 { Some((p[0].to_string(), (p[1].to_string(), p[2].to_string()))) } else { None } })
        .collect();
    let mut containers = Vec::new();
    for line in out.lines() {
        let p: Vec<&str> = line.split('\t').collect();
        if p.len() < 4 { continue; }
        let name = p.get(1).unwrap_or(&"-").to_string();
        let (cpu, mem) = stats_map.get(&name).cloned().unwrap_or_else(|| ("-".into(), "-".into()));
        containers.push(Container {
            id: p[0].to_string(), name,
            image: p.get(2).unwrap_or(&"-").to_string(),
            status: p.get(3).unwrap_or(&"-").to_string(),
            ports: p.get(4).unwrap_or(&"").to_string(), cpu, mem,
        });
    }
    containers
}

pub fn list_images() -> Vec<Image> {
    let (out, _) = docker(&["images", "--format", "{{.Repository}}\t{{.Tag}}\t{{.ID}}\t{{.Size}}"]);
    let mut images = Vec::new();
    for line in out.lines() {
        let p: Vec<&str> = line.split('\t').collect();
        if p.len() < 4 { continue; }
        images.push(Image { repo: p[0].to_string(), tag: p[1].to_string(), id: p[2].to_string(), size: p[3].to_string() });
    }
    images
}

pub fn container_action(id: &str, action: &str) -> Result<String, String> {
    let args: Vec<&str> = match action {
        "start" => vec!["start", id],
        "stop" => vec!["stop", id],
        "restart" => vec!["restart", id],
        "remove" => vec!["rm", "-f", id],
        _ => return Err("Unknown action".to_string()),
    };
    let (out, err) = docker(&args);
    if err { Err(out) } else { Ok(format!("{} {}", action, id)) }
}

pub fn container_logs(id: &str, tail: &str) -> String {
    let (out, _) = docker(&["logs", "--tail", tail, id]);
    out
}

pub fn container_inspect(id: &str) -> String {
    let (out, _) = docker(&["inspect", id]);
    out
}

pub fn list_compose_projects() -> Vec<ComposeProject> {
    let mut projects = Vec::new();
    // List all containers with compose labels
    let (out, _) = docker(&["ps", "-a", "--format", "{{.Label \"com.docker.compose.project\"}}\t{{.Label \"com.docker.compose.project.working_dir\"}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}"]);
    let mut project_map: std::collections::HashMap<String, (String, Vec<ComposeService>)> = std::collections::HashMap::new();
    for line in out.lines() {
        let p: Vec<&str> = line.split('\t').collect();
        if p.len() < 6 { continue; }
        let project = p[0].to_string();
        if project.is_empty() { continue; }
        let workdir = p[1].to_string();
        let service_name = p[2].to_string();
        // Strip compose prefix
        let clean_name = service_name.strip_prefix(&format!("{}-", project)).unwrap_or(&service_name).to_string();
        let svc = ComposeService {
            name: clean_name,
            image: p[3].to_string(),
            status: p[4].to_string(),
            ports: p[5].to_string(),
        };
        let entry = project_map.entry(project).or_insert((workdir, Vec::new()));
        entry.1.push(svc);
    }
    for (name, (path, services)) in project_map {
        let running = services.iter().filter(|s| s.status.contains("Up")).count();
        let total = services.len();
        projects.push(ComposeProject {
            name, path,
            status: format!("{}/{} running", running, total),
            services,
        });
    }
    projects
}

pub fn compose_action(project: &str, action: &str) -> Result<String, String> {
    let args: Vec<&str> = match action {
        "up" => vec!["-p", project, "up", "-d"],
        "down" => vec!["-p", project, "down"],
        "restart" => vec!["-p", project, "restart"],
        "stop" => vec!["-p", project, "stop"],
        "pull" => vec!["-p", project, "pull"],
        "logs" => vec!["-p", project, "logs", "--tail", "100"],
        _ => return Err("Unknown action".to_string()),
    };
    let (out, err) = compose(&args);
    if err { Err(out) } else { Ok(out) }
}

pub fn compose_logs(project: &str, tail: &str) -> String {
    let (out, _) = compose(&["-p", project, "logs", "--tail", tail]);
    out
}
