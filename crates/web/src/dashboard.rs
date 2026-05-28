use serde::Serialize;
use std::fs;
use std::process::Command;

#[derive(Serialize, Clone, Default)]
pub struct DashboardData {
    pub os: String, pub kernel: String, pub hostname: String, pub uptime: u64, pub arch: String,
    pub cpu_vendor: String, pub cpu_model: String, pub cpu_cores: u32, pub cpu_mhz: f64,
    pub cpu_pct: f64, pub cpu_cores_pct: Vec<f64>,
    pub load1: f64, pub load5: f64, pub load15: f64,
    pub mem_total: u64, pub mem_used: u64, pub mem_avail: u64,
    pub mem_buffers: u64, pub mem_cached: u64,
    pub swap_total: u64, pub swap_used: u64,
    pub disks: Vec<DiskInfo>, pub ifaces: Vec<NetIface>,
    pub ips: Vec<String>, pub ports: Vec<PortInfo>,
    pub procs: u32, pub procs_max: u32, pub fd_max: u64, pub fd_cur: u64,
    pub threads: u64, pub zombies: u32,
    pub top_cpu: Vec<ProcInfo>, pub top_mem: Vec<ProcInfo>,
    pub anomalies: Vec<String>, pub apps: Vec<AppInfo>,
    pub sys_errors: Vec<SysError>,
    pub disk_read_kb: u64, pub disk_write_kb: u64,
    pub hw_vendor: String, pub hw_model: String,
}

#[derive(Serialize, Clone)]
pub struct DiskInfo { pub dev: String, pub size: String, pub used: String, pub pct: String, pub mount: String }
#[derive(Serialize, Clone)]
pub struct NetIface { pub name: String, pub rx_bytes: u64, pub tx_bytes: u64 }
#[derive(Serialize, Clone)]
pub struct PortInfo { pub port: u16, pub proto: String, pub process: String, pub pid: u32 }
#[derive(Serialize, Clone)]
pub struct ProcInfo { pub name: String, pub pid: u32, pub cpu: f64, pub mem_kb: u64 }
#[derive(Serialize, Clone)]
pub struct AppInfo {
    pub name: String, pub pid: u32, pub ports: Vec<u16>, pub category: String,
    pub user: String, pub cpu_pct: f64, pub mem_kb: u64,
    pub service_name: String, pub exe_path: String, pub threads: u32,
}
#[derive(Serialize, Clone)]
pub struct SysError { pub service: String, pub message: String, pub severity: String, pub timestamp: String }

fn r(p: &str) -> String { fs::read_to_string(p).unwrap_or_default() }
fn val(l: &str) -> String { l.split(':').nth(1).unwrap_or("?").trim().to_string() }
fn is_virt(n: &str) -> bool {
    n=="lo"||n.starts_with("docker")||n.starts_with("veth")||n.starts_with("br-")||
    n.starts_with("virbr")||n.starts_with("tap")||n.starts_with("tun")||n.starts_with("cali")||
    n.starts_with("flannel")||n.starts_with("kube")||n.starts_with("cni")
}

pub fn gather() -> DashboardData {
    let rel = r("/etc/os-release");
    let mut os_name = "Linux".to_string();
    for l in rel.lines() { if l.starts_with("PRETTY_NAME=") { os_name = l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string(); } }
    let kernel = r("/proc/version").split_whitespace().take(3).collect::<Vec<_>>().join(" ");
    let hostname = r("/proc/sys/kernel/hostname").trim().to_string();
    let uptime = r("/proc/uptime").split_whitespace().next().unwrap_or("0").parse::<f64>().unwrap_or(0.0) as u64;
    let arch = std::env::consts::ARCH.to_string();

    let ci = r("/proc/cpuinfo");
    let mut cpu_vendor = String::new(); let mut cpu_model = String::new();
    let mut cpu_cores = 0u32; let mut cpu_mhz = 0f64;
    for l in ci.lines() {
        if l.starts_with("vendor_id") && cpu_vendor.is_empty() { cpu_vendor = val(l); }
        if l.starts_with("model name") && cpu_model.is_empty() { cpu_model = val(l); }
        if l.starts_with("processor") { cpu_cores += 1; }
        if l.starts_with("cpu MHz") && cpu_mhz == 0.0 { cpu_mhz = val(l).parse().unwrap_or(0.0); }
    }
    let cpu_pct = cpu_usage();
    let cpu_cores_pct = cpu_cores_usage();

    let la = r("/proc/loadavg"); let mut ls = la.split_whitespace();
    let load1: f64 = ls.next().unwrap_or("0").parse().unwrap_or(0.0);
    let load5: f64 = ls.next().unwrap_or("0").parse().unwrap_or(0.0);
    let load15: f64 = ls.next().unwrap_or("0").parse().unwrap_or(0.0);

    let (mem_total, mem_free, mem_avail, swap_total, swap_free, _buf_cache) = meminfo();
    let mem_used = mem_total.saturating_sub(mem_free);
    let (mem_buffers, mem_cached) = mem_detail();

    let mut disks = Vec::new();
    if let Ok(o) = Command::new("df").args(["-h","-x","tmpfs","-x","devtmpfs","-x","overlay","-x","squashfs"]).output() {
        for l in String::from_utf8_lossy(&o.stdout).lines().skip(1).take(8) {
            let c: Vec<&str> = l.split_whitespace().collect();
            if c.len() >= 6 { disks.push(DiskInfo { dev: c[0].to_string(), size: c[1].to_string(), used: c[2].to_string(), pct: c[4].to_string(), mount: c[5].to_string() }); }
        }
    }

    let mut ifaces = Vec::new();
    for l in r("/proc/net/dev").lines().skip(2) {
        let n = l.split(':').next().unwrap_or("").trim();
        if n.is_empty() || is_virt(n) { continue; }
        let p: Vec<&str> = l.split_whitespace().collect();
        let rx: u64 = p.get(1).unwrap_or(&"0").parse().unwrap_or(0);
        let tx: u64 = p.get(9).unwrap_or(&"0").parse().unwrap_or(0);
        ifaces.push(NetIface { name: n.to_string(), rx_bytes: rx, tx_bytes: tx });
    }
    let mut ips = Vec::new();
    if let Ok(o) = Command::new("hostname").args(["-I"]).output() {
        for ip in String::from_utf8_lossy(&o.stdout).trim().split_whitespace() { ips.push(ip.to_string()); }
    }
    let ports = get_ports();

    let procs = fs::read_dir("/proc").map(|e| e.filter_map(|x|x.ok()).filter(|x| x.file_type().map(|t|t.is_dir()).unwrap_or(false) && x.file_name().to_string_lossy().chars().all(|c|c.is_ascii_digit())).count() as u32).unwrap_or(0);
    let procs_max: u32 = r("/proc/sys/kernel/pid_max").trim().parse().unwrap_or(0);
    let fd_max: u64 = r("/proc/sys/fs/file-max").trim().parse().unwrap_or(0);
    let fd_cur = fs::read_dir("/proc/self/fd").map(|e| e.count() as u64).unwrap_or(0);
    let threads: u64 = r("/proc/loadavg").split_whitespace().nth(3).and_then(|s|s.split('/').nth(1)).unwrap_or("0").parse().unwrap_or(0);
    let zombies = count_zombies();
    let (top_cpu, top_mem) = top_procs();
    let apps = scan_apps(&ports);
    let sys_errors = scan_errors();
    let (disk_read_kb, disk_write_kb) = disk_io();

    let hw_vendor = r("/sys/class/dmi/id/sys_vendor").trim().to_string();
    let hw_model = r("/sys/class/dmi/id/product_name").trim().to_string();

    let mut anomalies = Vec::new();
    if cpu_pct > 90.0 { anomalies.push(format!("CPU {:.1}% 极高", cpu_pct)); }
    else if cpu_pct > 70.0 { anomalies.push(format!("CPU {:.1}% 偏高", cpu_pct)); }
    let mp = if mem_total > 0 { mem_used as f64 / mem_total as f64 * 100.0 } else { 0.0 };
    if mp > 95.0 { anomalies.push(format!("内存 {:.1}% 极高", mp)); }
    else if mp > 85.0 { anomalies.push(format!("内存 {:.1}% 偏高", mp)); }
    if swap_total > 0 { let sp = (swap_total - swap_free) as f64 / swap_total as f64 * 100.0;
        if sp > 80.0 { anomalies.push(format!("Swap {:.1}% 极高", sp)); } }
    if load1 > cpu_cores as f64 * 3.0 { anomalies.push(format!("负载 {:.2} 极高", load1)); }
    else if load1 > cpu_cores as f64 * 2.0 { anomalies.push(format!("负载 {:.2} 偏高", load1)); }
    if procs > 500 { anomalies.push(format!("进程数 {} 偏高", procs)); }
    if fd_max > 0 { let fd_pct = fd_cur as f64 / fd_max as f64 * 100.0;
        if fd_pct > 90.0 { anomalies.push(format!("FD {:.1}% 极高", fd_pct)); } }
    if zombies > 0 { anomalies.push(format!("僵尸进程 {}个", zombies)); }
    for p in top_mem.iter().take(3) { if p.mem_kb > 4*1024*1024 { anomalies.push(format!("{} 内存 {:.1}GB", p.name, p.mem_kb as f64/1048576.0)); } }

    DashboardData {
        os: os_name, kernel, hostname, uptime, arch, cpu_vendor, cpu_model, cpu_cores, cpu_mhz,
        cpu_pct, cpu_cores_pct, load1, load5, load15,
        mem_total: mem_total/1024, mem_used: mem_used/1024, mem_avail: mem_avail/1024,
        mem_buffers: mem_buffers/1024, mem_cached: mem_cached/1024,
        swap_total: swap_total/1024, swap_used: (swap_total-swap_free)/1024,
        disks, ifaces, ips, ports, procs, procs_max, fd_max, fd_cur, threads, zombies,
        top_cpu, top_mem, anomalies, apps, sys_errors, disk_read_kb, disk_write_kb,
        hw_vendor, hw_model,
    }
}

fn cpu_usage() -> f64 {
    let s = r("/proc/stat"); let mut us = 0u64; let mut to = 0u64;
    for l in s.lines() { if l.starts_with("cpu ") { for (i,v) in l.split_whitespace().skip(1).enumerate() { let x: u64 = v.parse().unwrap_or(0); if i<3 { us+=x; } to+=x; } } }
    if to > 0 { us as f64 / to as f64 * 100.0 } else { 0.0 }
}
fn cpu_cores_usage() -> Vec<f64> {
    let s = r("/proc/stat"); let mut cores = Vec::new();
    for l in s.lines() { if l.starts_with("cpu") && !l.starts_with("cpu ") {
        let mut us = 0u64; let mut to = 0u64;
        for (i,v) in l.split_whitespace().skip(1).enumerate() { let x: u64 = v.parse().unwrap_or(0); if i<3 { us+=x; } to+=x; }
        cores.push(if to>0 { us as f64 / to as f64 * 100.0 } else { 0.0 });
    } }
    cores
}
fn meminfo() -> (u64,u64,u64,u64,u64,u64) {
    let (mut mt,mut mf,mut ma,mut st,mut sf,mut bc) = (0u64,0u64,0u64,0u64,0u64,0u64);
    for l in r("/proc/meminfo").lines() { let v: u64 = l.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        if l.starts_with("MemTotal:"){mt=v;} if l.starts_with("MemFree:"){mf=v;} if l.starts_with("MemAvailable:"){ma=v;}
        if l.starts_with("SwapTotal:"){st=v;} if l.starts_with("SwapFree:"){sf=v;}
        if l.starts_with("Buffers:"){bc+=v;} if l.starts_with("Cached:"){bc+=v;}
    }
    (mt,mf,ma,st,sf,bc)
}
fn mem_detail() -> (u64, u64) {
    let (mut buf, mut cached) = (0u64, 0u64);
    for l in r("/proc/meminfo").lines() { let v: u64 = l.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        if l.starts_with("Buffers:") { buf = v; } if l.starts_with("Cached:") { cached = v; }
    }
    (buf, cached)
}
fn disk_io() -> (u64, u64) {
    let (mut read_kb, mut write_kb) = (0u64, 0u64);
    for l in r("/proc/diskstats").lines() {
        let p: Vec<&str> = l.split_whitespace().collect();
        if p.len() < 14 { continue; }
        let name = p.get(2).unwrap_or(&"");
        if name.starts_with("loop") || name.starts_with("ram") { continue; }
        read_kb += p.get(5).unwrap_or(&"0").parse::<u64>().unwrap_or(0) * 512 / 1024;
        write_kb += p.get(9).unwrap_or(&"0").parse::<u64>().unwrap_or(0) * 512 / 1024;
    }
    (read_kb, write_kb)
}
fn get_ports() -> Vec<PortInfo> {
    let mut ports = Vec::new();
    if let Ok(o) = Command::new("ss").args(["-tlnp"]).output() {
        for l in String::from_utf8_lossy(&o.stdout).lines().skip(1) {
            if !l.contains("LISTEN") { continue; }
            let p: Vec<&str> = l.split_whitespace().collect();
            if p.len() < 6 { continue; }
            let local = p.get(3).unwrap_or(&":");
            let port_str = local.rsplit(':').next().unwrap_or("0");
            let clean_port = port_str.split('%').next().unwrap_or("0");
            let proto = if p[0].starts_with("tcp") { "TCP" } else { "UDP" };
            let mut proc_name = String::new(); let mut pid = 0u32;
            if let Some(proc_part) = p.get(5) {
                if let Some(s) = proc_part.find("((\"") { let rest = &proc_part[s+3..]; if let Some(e) = rest.find("\",pid=") { proc_name = rest[..e].to_string(); } }
                if let Some(s) = proc_part.find("pid=") { let rest = &proc_part[s+4..]; pid = rest.split(|c:char|!c.is_ascii_digit()).next().unwrap_or("0").parse().unwrap_or(0); }
            }
            if let Ok(po) = clean_port.parse::<u16>() {
                if let Some(existing) = ports.iter_mut().find(|x:&&mut PortInfo| x.port==po) {
                    if !proc_name.is_empty() && existing.process.is_empty() { existing.process=proc_name; existing.pid=pid; }
                } else { ports.push(PortInfo{port:po,proto:proto.to_string(),process:proc_name,pid}); }
            }
        }
    }
    ports
}
fn count_zombies() -> u32 {
    let mut z = 0u32;
    if let Ok(dir) = fs::read_dir("/proc") { for e in dir.filter_map(|x|x.ok()) {
        if !e.file_name().to_string_lossy().chars().all(|c|c.is_ascii_digit()) { continue; }
        let stat = r(&format!("/proc/{}/stat", e.file_name().to_string_lossy()));
        let state = stat.splitn(2,')').nth(1).unwrap_or("").split_whitespace().next().unwrap_or("");
        if state == "Z" { z += 1; }
    } }
    z
}
fn top_procs() -> (Vec<ProcInfo>, Vec<ProcInfo>) {
    let mut cl = Vec::new(); let mut ml = Vec::new();
    if let Ok(dir) = fs::read_dir("/proc") { for e in dir.filter_map(|x|x.ok()) {
        let ns = e.file_name().to_string_lossy().into_owned();
        if !ns.chars().all(|c|c.is_ascii_digit()) { continue; }
        let pid: u32 = ns.parse().unwrap_or(0); if pid == 0 { continue; }
        let pd = format!("/proc/{}", ns);
        let stat = r(&format!("{}/stat", pd)); if stat.is_empty() { continue; }
        let name = if let Some(s) = stat.find('(') { if let Some(e) = stat.rfind(')') { stat[s+1..e].to_string() } else { String::new() } } else { String::new() };
        let after: Vec<&str> = stat.splitn(2,')').nth(1).unwrap_or("").split_whitespace().collect();
        if after.len() < 13 { continue; }
        let ut: u64 = after[11].parse().unwrap_or(0); let st: u64 = after[12].parse().unwrap_or(0);
        let mem_kb: u64 = r(&format!("{}/statm", pd)).split_whitespace().nth(1).and_then(|s|s.parse::<u64>().ok()).unwrap_or(0) * 4;
        cl.push(ProcInfo{name:name.clone(),pid,cpu:(ut+st) as f64,mem_kb});
        ml.push(ProcInfo{name,pid,cpu:(ut+st) as f64,mem_kb});
    } }
    cl.sort_by(|a,b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal)); cl.truncate(10);
    ml.sort_by(|a,b| b.mem_kb.cmp(&a.mem_kb)); ml.truncate(10);
    (cl, ml)
}
fn scan_apps(ports: &[PortInfo]) -> Vec<AppInfo> {
    use std::collections::HashSet;
    let mut seen: HashSet<u32> = HashSet::new(); let mut apps = Vec::new();
    if let Ok(dir) = fs::read_dir("/proc") { for e in dir.filter_map(|x|x.ok()) {
        let ns = e.file_name().to_string_lossy().into_owned();
        if !ns.chars().all(|c|c.is_ascii_digit()) { continue; }
        let pid: u32 = ns.parse().unwrap_or(0); if pid == 0 || seen.contains(&pid) { continue; }
        let pd = format!("/proc/{}", ns);
        let stat = r(&format!("{}/stat", pd)); if stat.is_empty() { continue; }
        let cmd = if let Some(s) = stat.find('(') { if let Some(e) = stat.rfind(')') { stat[s+1..e].to_string() } else { String::new() } } else { String::new() };
        let after: Vec<&str> = stat.splitn(2,')').nth(1).unwrap_or("").split_whitespace().collect();
        if after.len() < 13 { continue; }
        let ut: u64 = after[11].parse().unwrap_or(0); let st: u64 = after[12].parse().unwrap_or(0);
        let mem: u64 = r(&format!("{}/status", pd)).lines().find(|l|l.starts_with("VmRSS:")).and_then(|l|l.split_whitespace().nth(1).unwrap_or("0").parse().ok()).unwrap_or(0);
        let cmdline = r(&format!("{}/cmdline", pd)).replace('\0', " ");
        let full_cmd = if cmdline.trim().is_empty() { cmd.clone() } else { cmdline.trim().to_string() };
        let pid_ports: Vec<u16> = ports.iter().filter(|p| p.pid == pid).map(|p| p.port).collect();
        let cat = categorize(&cmd, &full_cmd);
        if cat == "Other" { continue; }
        seen.insert(pid);
        let uid = after.get(2).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let user = get_user(uid);
        let exe_path = fs::read_link(format!("{}/exe", pd)).ok().map(|p|p.to_string_lossy().to_string()).unwrap_or_default();
        let service_name = get_service_name(pid, &cmd);
        let threads_count = after.get(17).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        apps.push(AppInfo { name: full_cmd, pid, ports: pid_ports, category: cat, user, cpu_pct: (ut+st) as f64, mem_kb: mem, service_name, exe_path, threads: threads_count });
    } }
    apps.sort_by(|a,b| category_order(&a.category).cmp(&category_order(&b.category)).then(b.ports.first().unwrap_or(&0).cmp(a.ports.first().unwrap_or(&0))));
    apps.truncate(20); apps
}
fn categorize(name: &str, full_cmd: &str) -> String {
    let s = format!("{} {}", name.to_lowercase(), full_cmd.to_lowercase());
    if s.contains("java")||s.contains("tomcat")||s.contains("jetty")||s.contains("spring")||s.contains("jar") { return "Java".into(); }
    if s.contains("nginx")||s.contains("apache")||s.contains("httpd")||s.contains("caddy")||s.contains("traefik")||s.contains("haproxy") { return "WebServer".into(); }
    if s.contains("mysqld")||s.contains("mariadb")||s.contains("postgres")||s.contains("mongod")||s.contains("clickhouse") { return "Database".into(); }
    if s.contains("redis")||s.contains("valkey")||s.contains("memcached")||s.contains("keydb") { return "Cache".into(); }
    if s.contains("kafka")||s.contains("rabbitmq")||s.contains("pulsar")||s.contains("nats") { return "MQ".into(); }
    if s.contains("elasticsearch")||s.contains("kibana")||s.contains("logstash")||s.contains("solr") { return "Search".into(); }
    if s.contains("docker")||s.contains("containerd")||s.contains("k3s")||s.contains("kube") { return "Container".into(); }
    "Other".into()
}
fn category_order(c: &str) -> u8 {
    match c { "Java"=>1,"WebServer"=>2,"Database"=>3,"Cache"=>4,"Search"=>5,"MQ"=>6,"Container"=>7,_=>8 }
}
fn get_user(uid: u32) -> String {
    for l in r("/etc/passwd").lines() { let parts: Vec<&str> = l.split(':').collect(); if parts.len()>=3 && parts[2].parse::<u32>().unwrap_or(0)==uid { return parts[0].to_string(); } }
    uid.to_string()
}
fn get_service_name(pid: u32, cmd: &str) -> String {
    let cgroup = r(&format!("/proc/{}/cgroup", pid));
    for line in cgroup.lines() { if let Some(idx) = line.find(".service") { let before = &line[..idx]; if let Some(slash) = before.rfind('/') { return before[slash+1..].to_string(); } } }
    cmd.split('/').last().unwrap_or("-").split_whitespace().next().unwrap_or("-").to_string()
}
fn scan_errors() -> Vec<SysError> {
    let mut errors = Vec::new();
    if let Ok(o) = Command::new("journalctl").args(["-p","err","--since","5min ago","--no-pager","-n","10"]).output() {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            if line.trim().is_empty()||line.contains("No entries")||line.contains("Boot ") { continue; }
            let parts: Vec<&str> = line.splitn(6, ' ').collect();
            let ts = if parts.len()>=3 { format!("{} {} {}", parts[0], parts[1], parts[2]) } else { String::new() };
            let rest = parts.get(5).copied().unwrap_or(line);
            let (service, _) = if let Some(bs) = rest.find('[') { (rest[..bs].rsplit(' ').next().unwrap_or("?").to_string(), String::new()) } else { (rest.split(':').next().unwrap_or("?").trim().to_string(), String::new()) };
            let msg = if let Some(colon) = rest.find(": ") { rest[colon+2..].to_string() } else { rest.to_string() };
            let short_msg = if msg.len()>100 { format!("{}...", &msg[..97]) } else { msg };
            let sev = if short_msg.to_lowercase().contains("critical")||short_msg.to_lowercase().contains("fatal") { "critical" } else if short_msg.to_lowercase().contains("error")||short_msg.to_lowercase().contains("fail") { "error" } else { "warning" };
            errors.push(SysError { service, message: short_msg, severity: sev.to_string(), timestamp: ts });
        }
    }
    errors.truncate(10); errors
}
