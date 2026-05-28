use std::fs;
use std::time::Instant;

pub struct Dashboard { pub data: Data, history: Vec<u64>, prev_cpu: (u64, u64), prev_net: (f64, f64, Instant), last: Instant, prev_procs: Vec<(u32,u64)> }

#[derive(Clone)]
pub struct Data {
    // OS
    pub os: String, pub kernel: String, pub hostname: String, pub uptime: u64, pub arch: String,
    // CPU detail
    pub cpu_vendor: String, pub cpu_model: String, pub cpu_cores: u32, pub cpu_mhz: f64,
    pub cpu_cache: String, pub cpu_flags: String, pub cpu_bogomips: f64, pub cpu_addr: String,
    // Load & usage
    pub load1: f64, pub load5: f64, pub load15: f64, pub cpu_pct: f64,
    // Memory
    pub mem_total: u64, pub mem_used: u64, pub mem_avail: u64,
    pub swap_total: u64, pub swap_used: u64, pub buf_cache: u64,
    // Disk
    pub disks: Vec<(String, String, String, String)>, // dev, size, used, pct
    // Network
    pub net_rx_rate: f64, pub net_tx_rate: f64, pub ifaces: Vec<(String, u64, u64)>,
    pub ips: Vec<String>, pub iface_ips: Vec<(String,String)>,
    // Ports with process info
    pub ports: Vec<PortInfo>,
    // Resources
    pub procs: u32, pub procs_max: u32, pub fd_max: u64, pub fd_cur: u64, pub threads: u64, pub zombies: u32,
    pub hw_vendor: String, pub hw_serial: String,
    // Top lists with anomaly flags
    pub top_cpu: Vec<ProcInfo>, pub top_mem: Vec<ProcInfo>,
    // Anomalies
    pub anomalies: Vec<String>,
    pub cpu_min: f64, pub cpu_max: f64, pub mem_min: u64, pub mem_max: u64,
    pub apps: Vec<AppInfo>,
    pub sys_errors: Vec<SysError>,
}

#[derive(Clone)]
pub struct SysError {
    pub service: String, pub pid: String, pub path: String,
    pub message: String, pub severity: ErrorSeverity, pub timestamp: String,
}

#[derive(Clone, PartialEq)]
pub enum ErrorSeverity { Critical, Error, Warning }

#[derive(Clone)]
pub struct AppInfo {
    pub name: String, pub pid: u32, pub ports: Vec<u16>,
    pub cpu: f64, pub mem_kb: u64,
    pub category: AppCategory, pub running: bool,
    pub user: String, pub threads: u32, pub cpu_pct: f64, pub mem_pct: f64,
    pub service_name: String,
}

#[derive(Clone, PartialEq)]
pub enum AppCategory { Java, WebServer, Database, Cache, MessageQueue, Search, Container, Other }

#[derive(Clone)]
pub struct PortInfo { pub port: u16, pub proto: String, pub process: String, pub pid: u32 }

#[derive(Clone)]
pub struct ProcInfo { pub name: String, pub pid: u32, pub cpu: f64, pub mem_kb: u64, pub anomaly: bool }

impl Dashboard {
    pub fn new() -> Self { let mut d=gather(); d.cpu_min=100.0; d.cpu_max=0.0; d.mem_min=u64::MAX; d.mem_max=0; Self{data:d,history:vec![0;30],prev_cpu:cpu_delta(),prev_net:net_nums(),last:Instant::now(),prev_procs:Vec::new()} }

    pub fn tick(&mut self) {
        if self.last.elapsed().as_millis() < 1000 { return; }
        let cur = cpu_delta();
        let pct = if cur.1 > self.prev_cpu.1 { ((cur.0 - self.prev_cpu.0) as f64 / (cur.1 - self.prev_cpu.1) as f64 * 100.0).min(100.0) } else { 0.0 };
        self.prev_cpu = cur;
        let cn = net_nums();
        let el = cn.2.duration_since(self.prev_net.2).as_secs_f64().max(0.5);
        let rx = ((cn.0 - self.prev_net.0) / el).max(0.0);
        let tx = ((cn.1 - self.prev_net.1) / el).max(0.0);
        self.prev_net = cn;
        let mut d = gather();
        d.cpu_pct = pct; d.net_rx_rate = rx; d.net_tx_rate = tx;
        let mut anoms = Vec::new();
        // CPU anomaly
        if pct > 90.0 { anoms.push(format!("CPU {:.1}% 极高(>90%)", pct)); }
        else if pct > 70.0 { anoms.push(format!("CPU {:.1}% 偏高(>70%)", pct)); }
        // Memory anomaly
        let mp = if d.mem_total>0{d.mem_used as f64/d.mem_total as f64*100.0}else{0.0};
        if mp>95.0{anoms.push(format!("内存 {:.1}% 极高(>95%)",mp));}
        else if mp>85.0{anoms.push(format!("内存 {:.1}% 偏高(>85%)",mp));}
        // Swap anomaly
        let sp=if d.swap_total>0{d.swap_used as f64/d.swap_total as f64*100.0}else{0.0};
        if sp>80.0{anoms.push(format!("Swap {:.1}% 极高(>80%)",sp));}
        else if sp>50.0{anoms.push(format!("Swap {:.1}% 偏高(>50%)",sp));}
        // Load average anomaly
        if d.load1>d.cpu_cores as f64*3.0{anoms.push(format!("负载 {:.2} 极高({}核)",d.load1,d.cpu_cores));}
        else if d.load1>d.cpu_cores as f64*2.0{anoms.push(format!("负载 {:.2} 偏高({}核)",d.load1,d.cpu_cores));}
        // Root partition anomaly
        let rp:f64=d.disks.iter().find(|(d,_,_,_)|d=="/dev/sda2"||d=="/dev/sda1"||d=="/dev/vda1"||d=="/dev/nvme0n1p1"||d=="/").map(|(_,_,_,p)|p.trim_end_matches('%').parse().unwrap_or(0.0)).unwrap_or(0.0);
        if rp>95.0{anoms.push(format!("根分区 {:.1}% 极高(>95%)",rp));}
        else if rp>80.0{anoms.push(format!("根分区 {:.1}% 偏高(>80%)",rp));}
        // High memory processes
        for p in d.top_mem.iter().take(5){
            if p.mem_kb>4*1024*1024{anoms.push(format!("{} PID:{} 内存:{:.1}GB 极高",p.name,p.pid,p.mem_kb as f64/1048576.0));}
            else if p.mem_kb>2*1024*1024{anoms.push(format!("{} PID:{} 内存:{:.1}GB 偏高",p.name,p.pid,p.mem_kb as f64/1048576.0));}
        }
        // Zombie processes
        if d.zombies>10{anoms.push(format!("僵尸进程 {}个 极高",d.zombies));}
        else if d.zombies>0{anoms.push(format!("僵尸进程 {}个",d.zombies));}
        // FD usage
        if d.fd_max>0{
            let fd_pct=d.fd_cur as f64/d.fd_max as f64*100.0;
            if fd_pct>90.0{anoms.push(format!("FD使用率 {:.1}% 极高",fd_pct));}
            else if fd_pct>80.0{anoms.push(format!("FD使用率 {:.1}% 偏高",fd_pct));}
        }
        // High CPU processes (per-process anomaly)
        for p in d.top_cpu.iter().take(3){
            if p.cpu>80.0{anoms.push(format!("{} PID:{} CPU:{:.1}% 极高",p.name,p.pid,p.cpu));}
        }
        // Network anomalies: high traffic
        if d.net_rx_rate>100_000_000.0{anoms.push(format!("入站流量 {:.1}MB/s 偏高",d.net_rx_rate/1_000_000.0));}
        if d.net_tx_rate>100_000_000.0{anoms.push(format!("出站流量 {:.1}MB/s 偏高",d.net_tx_rate/1_000_000.0));}
        // Disk I/O anomaly: multiple disks > 90%
        let high_disks: Vec<&str> = d.disks.iter().filter(|(_,_,_,p)|{
            p.trim_end_matches('%').parse::<f64>().unwrap_or(0.0) > 90.0
        }).map(|(n,_,_,_)| n.as_str()).collect();
        if high_disks.len() > 1 {
            anoms.push(format!("多磁盘告警: {}", high_disks.join(", ")));
        }
        // Process count anomaly
        if d.procs > 500 { anoms.push(format!("进程数 {} 偏高", d.procs)); }
        d.cpu_min = self.data.cpu_min.min(pct); d.cpu_max = self.data.cpu_max.max(pct);
        d.mem_min = self.data.mem_min.min(d.mem_used).max(1); d.mem_max = self.data.mem_max.max(d.mem_used);
        // Compute per-process CPU% from deltas (CLK_TCK=100, 1s interval → delta_ticks ≈ CPU%)
        let raw_ticks: Vec<(u32,u64)> = d.top_cpu.iter().map(|p| (p.pid, p.cpu as u64)).collect();
        for p in &mut d.top_cpu {
            let cur_total = p.cpu as u64;
            let prev_total = self.prev_procs.iter().find(|(pid,_)| *pid == p.pid).map(|(_,v)| *v).unwrap_or(cur_total);
            let delta = cur_total.saturating_sub(prev_total);
            p.cpu = delta as f64;
        }
        self.prev_procs = raw_ticks;
        d.top_cpu.sort_by(|a,b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
        d.top_mem.sort_by(|a,b| b.mem_kb.cmp(&a.mem_kb));
        d.anomalies=anoms;
        self.data=d;
        self.history.push(pct as u64); if self.history.len() > 30 { self.history.remove(0); }
        self.last = Instant::now();
    }
    pub fn sparkline(&self) -> Vec<u64> { let h:Vec<_>=self.history.iter().rev().take(20).copied().collect(); h.into_iter().rev().collect() }
}

fn r(p:&str)->String{fs::read_to_string(p).unwrap_or_default()}

fn cpu_delta() -> (u64, u64) {
    let s=r("/proc/stat"); let mut us=0u64; let mut to=0u64;
    for l in s.lines(){if l.starts_with("cpu "){for(i,v)in l.split_whitespace().skip(1).enumerate(){let x:u64=v.parse().unwrap_or(0);if i<3{us+=x;}to+=x;}}}
    (us,to.max(1))
}

fn net_nums() -> (f64, f64, Instant) {
    let (mut rx,mut tx)=(0f64,0f64);
    for l in r("/proc/net/dev").lines().skip(2){let n=l.split(':').next().unwrap_or("").trim();if n.is_empty()||is_virt(n){continue;}let p:Vec<&str>=l.split_whitespace().collect();rx+=p.get(1).unwrap_or(&"0").parse::<f64>().unwrap_or(0.0);tx+=p.get(9).unwrap_or(&"0").parse::<f64>().unwrap_or(0.0);}
    (rx,tx,Instant::now())
}

#[cfg(target_os="linux")]
fn gather() -> Data {
    // OS
    let rel=r("/etc/os-release"); let mut os="Linux".into();
    for l in rel.lines(){if l.starts_with("PRETTY_NAME="){os=l.trim_start_matches("PRETTY_NAME=").trim_matches('"').into();}}
    let k=r("/proc/version").split_whitespace().take(3).collect::<Vec<_>>().join(" ");
    let hn=r("/proc/sys/kernel/hostname").trim().to_string();
    let up=r("/proc/uptime").split_whitespace().next().unwrap_or("0").parse::<f64>().unwrap_or(0.0) as u64;
    let arch = std::env::consts::ARCH.to_string();

    // CPU detail from /proc/cpuinfo
    let ci=r("/proc/cpuinfo");
    let mut vendor=String::new(); let mut model=String::new(); let mut cores=0u32; let mut mhz=0f64;
    let mut cache=String::new(); let mut flags=String::new(); let mut bogomips=0f64; let mut addr=String::new();
    for l in ci.lines(){
        if l.starts_with("vendor_id") && vendor.is_empty() { vendor = val(l); }
        if l.starts_with("model name") && model.is_empty() { model = val(l); }
        if l.starts_with("processor") { cores += 1; }
        if l.starts_with("cpu MHz") && mhz == 0.0 { mhz = val(l).parse().unwrap_or(0.0); }
        if l.starts_with("cache size") && cache.is_empty() { cache = val(l); }
        if l.starts_with("flags") && flags.is_empty() { flags = val(l); }
        if l.starts_with("bogomips") && bogomips == 0.0 { bogomips = val(l).parse().unwrap_or(0.0); }
        if l.starts_with("address sizes") && addr.is_empty() { addr = val(l); }
    }

    // Load
    let la=r("/proc/loadavg"); let mut ls=la.split_whitespace();
    let l1:f64=ls.next().unwrap_or("0").parse().unwrap_or(0.0);
    let l2:f64=ls.next().unwrap_or("0").parse().unwrap_or(0.0);
    let l3:f64=ls.next().unwrap_or("0").parse().unwrap_or(0.0);

    // Memory
    let(mt,mf,ma,st,sf,bc)=meminfo();

    // Disk
    let mut disks=Vec::new();
    if let Ok(o)=std::process::Command::new("df").args(["-h","-x","tmpfs","-x","devtmpfs","-x","overlay","-x","squashfs"]).output(){
        for l in String::from_utf8_lossy(&o.stdout).lines().skip(1).take(8){let c:Vec<&str>=l.split_whitespace().collect();if c.len()>=6{disks.push((c[0].into(),c[1].into(),c[2].into(),c[4].into()));}}
    }

    // Network interfaces with bytes
    let mut ifaces=Vec::new();
    for l in r("/proc/net/dev").lines().skip(2){let n=l.split(':').next().unwrap_or("").trim();if n.is_empty()||is_virt(n){continue;}let p:Vec<&str>=l.split_whitespace().collect();let rx=p.get(1).unwrap_or(&"0").parse::<u64>().unwrap_or(0);let tx=p.get(9).unwrap_or(&"0").parse::<u64>().unwrap_or(0);ifaces.push((n.to_string(),rx,tx));}
    // Server IPs
    let mut ips=Vec::new();let mut iface_ips=Vec::new();
    if let Ok(o)=std::process::Command::new("hostname").args(["-I"]).output(){
        for ip in String::from_utf8_lossy(&o.stdout).trim().split_whitespace(){ ips.push(ip.to_string()); }
    }
    // Per-interface IPs from ip addr
    if let Ok(o)=std::process::Command::new("ip").args(["-4","-br","addr","show"]).output(){
        for l in String::from_utf8_lossy(&o.stdout).lines(){
            let p:Vec<&str>=l.split_whitespace().collect();
            if p.len()>=3 && p[1]=="UP" && !is_virt(p[0]) { iface_ips.push((p[0].to_string(),p[2].to_string())); }
        }
    }

    // Ports with process info
    let mut ports=Vec::new();
    if let Ok(o)=std::process::Command::new("ss").args(["-tlnp"]).output(){
        for l in String::from_utf8_lossy(&o.stdout).lines().skip(1){
            if !l.contains("LISTEN"){continue;}
            let p:Vec<&str>=l.split_whitespace().collect();
            // ss output: State Recv-Q Send-Q LocalAddress:Port PeerAddress:Port Process
            // Data:      LISTEN 0      4096   127.0.0.1:6012    0.0.0.0:*       users:(("sshd",pid=552089,fd=7))
            if p.len() < 6 { continue; }
            let local = p.get(3).unwrap_or(&":"); // LocalAddress:Port is field index 3
            let port_str = local.rsplit(':').next().unwrap_or("0");
            // Remove %iface suffix like 127.0.0.53%lo:53
            let clean_port = port_str.split('%').next().unwrap_or("0");
            let proto = if p[0].starts_with("tcp") {"TCP"} else {"UDP"};
            let mut proc_name=String::new(); let mut pid=0u32;
            if let Some(proc_part)=p.get(5){
                // Parse: users:(("sshd",pid=552089,fd=7)) or users:(("node /opt/node-",pid=161280,fd=24))
                if let Some(s)=proc_part.find("((\"") {
                    let rest = &proc_part[s+3..];
                    if let Some(e)=rest.find("\",pid=") {
                        proc_name = rest[..e].to_string();
                    }
                }
                if let Some(s)=proc_part.find("pid=") {
                    let rest = &proc_part[s+4..];
                    pid = rest.split(|c:char| !c.is_ascii_digit()).next().unwrap_or("0").parse().unwrap_or(0);
                }
            }
            if let Ok(po)=clean_port.parse::<u16>(){
                // Dedup - keep the one with process info
                if let Some(existing)=ports.iter_mut().find(|x:&&mut PortInfo| x.port==po) {
                    if !proc_name.is_empty() && existing.process.is_empty() { existing.process=proc_name; existing.pid=pid; }
                } else {
                    ports.push(PortInfo{port:po,proto:proto.to_string(),process:proc_name,pid});
                }
            }
        }
    }
    // Sort ports: important services first, then by port number
    ports.sort_by(|a, b| {
        let pa = port_priority(&a.process);
        let pb = port_priority(&b.process);
        pb.cmp(&pa).then(a.port.cmp(&b.port))
    });

    // Process info
    let pc=fs::read_dir("/proc").map(|e|e.filter_map(|x|x.ok()).filter(|x|x.file_type().map(|t|t.is_dir()).unwrap_or(false)&&x.file_name().to_string_lossy().chars().all(|c|c.is_ascii_digit())).count() as u32).unwrap_or(0);
    let fm=r("/proc/sys/fs/file-max").trim().parse().unwrap_or(0u64);
    let fc=fs::read_dir("/proc/self/fd").map(|e|e.count() as u64).unwrap_or(0);
    let threads:u64 = r("/proc/loadavg").split_whitespace().nth(3).and_then(|s|s.split('/').nth(1)).unwrap_or("0").parse().unwrap_or(0);

    let hw_v = r("/sys/class/dmi/id/sys_vendor").trim().to_string();
    let hw_s = r("/sys/class/dmi/id/product_serial").trim().to_string();
    let pid_max: u32 = r("/proc/sys/kernel/pid_max").trim().parse().unwrap_or(0);
    let apps = scan_apps(&ports);
    let sys_errors = scan_journal_errors();
    let (tc,tm)=top_procs();

    Data{os,kernel:k,hostname:hn,uptime:up,arch,cpu_vendor:cpu_vendor(),cpu_model:model,cpu_cores:cores,cpu_mhz:mhz,cpu_cache:cache,cpu_flags:flags,cpu_bogomips:bogomips,cpu_addr:addr,
        load1:l1,load5:l2,load15:l3,cpu_pct:0.0,
        mem_total:mt/1024,mem_used:(mt-mf)/1024,mem_avail:ma/1024,swap_total:st/1024,swap_used:(st-sf)/1024,buf_cache:bc/1024,
        disks,net_rx_rate:0.0,net_tx_rate:0.0,ifaces,ips,iface_ips,ports,procs:pc,procs_max:pid_max,fd_max:fm,fd_cur:fc,threads,zombies:count_zombies(),hw_vendor:hw_v,hw_serial:hw_s,cpu_min:100.0,cpu_max:0.0,mem_min:u64::MAX,mem_max:0,apps,sys_errors,
        top_cpu:tc,top_mem:tm,anomalies:Vec::new()}
}

fn val(l:&str)->String{l.split(':').nth(1).unwrap_or("?").trim().to_string()}

fn cpu_vendor() -> String {
    let ci = r("/proc/cpuinfo");
    for l in ci.lines() { if l.starts_with("vendor_id") { return val(l); } }
    String::new()
}

fn meminfo() -> (u64,u64,u64,u64,u64,u64) {
    let(mut mt,mut mf,mut ma,mut st,mut sf,mut bc)=(0u64,0u64,0u64,0u64,0u64,0u64);
    for l in r("/proc/meminfo").lines(){let v=l.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        if l.starts_with("MemTotal:"){mt=v;}if l.starts_with("MemFree:"){mf=v;}if l.starts_with("MemAvailable:"){ma=v;}
        if l.starts_with("SwapTotal:"){st=v;}if l.starts_with("SwapFree:"){sf=v;}
        if l.starts_with("Buffers:"){bc+=v;}if l.starts_with("Cached:"){bc+=v;}
    }
    (mt,mf,ma,st,sf,bc)
}

fn top_procs() -> (Vec<ProcInfo>, Vec<ProcInfo>) {
    let mut cl=Vec::new(); let mut ml=Vec::new();
    if let Ok(dir)=fs::read_dir("/proc"){
        for e in dir.filter_map(|x|x.ok()){
            let ns=e.file_name().to_string_lossy().into_owned();
            if !ns.chars().all(|c|c.is_ascii_digit()){continue;}
            let pid:u32=ns.parse().unwrap_or(0); if pid==0{continue;}
            let pd=format!("/proc/{}",ns);
            let stat=r(&format!("{}/stat",pd));
            if stat.is_empty(){continue;}
            let name=if let Some(s)=stat.find('('){if let Some(e)=stat.rfind(')'){stat[s+1..e].to_string()}else{String::new()}}else{String::new()};
            let after:Vec<&str>=stat.splitn(2,')').nth(1).unwrap_or("").split_whitespace().collect();
            if after.len()<13{continue;}
            let ut:u64=after[11].parse().unwrap_or(0); let st:u64=after[12].parse().unwrap_or(0);
            let mem_kb:u64=r(&format!("{}/status",pd)).lines().find(|l|l.starts_with("VmRSS:")).and_then(|l|l.split_whitespace().nth(1).unwrap_or("0").parse().ok()).unwrap_or(0);
            let cpu_ticks=ut+st;
            let anomaly = cpu_ticks > 10000 || mem_kb > 500 * 1024;
            cl.push(ProcInfo{name:name.clone(),pid,cpu:cpu_ticks as f64,mem_kb,anomaly});
            ml.push(ProcInfo{name,pid,cpu:cpu_ticks as f64,mem_kb,anomaly:mem_kb>500*1024});
        }
    }
    // Always include self as fallback
    let self_stat = r("/proc/self/stat");
    if !self_stat.is_empty() {
        let self_name = if let Some(s)=self_stat.find('('){if let Some(e)=self_stat.rfind(')'){self_stat[s+1..e].to_string()}else{String::new()}}else{String::new()};
        let after_self: Vec<&str> = self_stat.splitn(2,')').nth(1).unwrap_or("").split_whitespace().collect();
        if after_self.len() >= 13 {
            let self_ut: u64 = after_self[11].parse().unwrap_or(0);
            let self_st: u64 = after_self[12].parse().unwrap_or(0);
            let self_mem: u64 = r("/proc/self/status").lines().find(|l|l.starts_with("VmRSS:")).and_then(|l|l.split_whitespace().nth(1).unwrap_or("0").parse().ok()).unwrap_or(0);
            cl.push(ProcInfo{name:self_name.clone(),pid:std::process::id(),cpu:(self_ut+self_st) as f64,mem_kb:self_mem,anomaly:false});
            ml.push(ProcInfo{name:self_name,pid:std::process::id(),cpu:(self_ut+self_st) as f64,mem_kb:self_mem,anomaly:false});
        }
    }
    cl.sort_by(|a,b|b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal)); cl.truncate(10);
    ml.sort_by(|a,b|b.mem_kb.cmp(&a.mem_kb)); ml.truncate(10);
    (cl,ml)
}

#[cfg(not(target_os="linux"))] fn gather() -> Data { Data{os:"Unsupported".into(),kernel:String::new(),hostname:String::new(),uptime:0,arch:String::new(),cpu_vendor:String::new(),cpu_model:String::new(),cpu_cores:0,cpu_mhz:0.0,cpu_cache:String::new(),cpu_flags:String::new(),cpu_bogomips:0.0,cpu_addr:String::new(),load1:0.0,load5:0.0,load15:0.0,cpu_pct:0.0,mem_total:0,mem_used:0,mem_avail:0,swap_total:0,swap_used:0,buf_cache:0,disks:vec![],net_rx_rate:0.0,net_tx_rate:0.0,ifaces:vec![],ips:vec![],iface_ips:vec![],ports:vec![],procs:0,procs_max:0,fd_max:0,fd_cur:0,threads:0,zombies:0,hw_vendor:String::new(),hw_serial:String::new(),top_cpu:vec![],top_mem:vec![],anomalies:vec![],cpu_min:0.0,cpu_max:0.0,mem_min:0,mem_max:0,apps:vec![],sys_errors:vec![]} }

fn count_zombies() -> u32 {
    let mut z = 0u32;
    if let Ok(dir) = fs::read_dir("/proc") {
        for e in dir.filter_map(|x| x.ok()) {
            if !e.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit()) { continue; }
            let stat = r(&format!("/proc/{}/stat", e.file_name().to_string_lossy()));
            let state = stat.splitn(2,')').nth(1).unwrap_or("").split_whitespace().next().unwrap_or("");
            if state == "Z" { z += 1; }
        }
    }
    z
}

fn port_priority(proc_name: &str) -> u32 {
    let name = proc_name.to_lowercase();
    if name.contains("java") || name.contains("tomcat") || name.contains("jetty") || name.contains("spring") { return 100; }
    if name.contains("nginx") || name.contains("apache") || name.contains("httpd") || name.contains("caddy") || name.contains("traefik") { return 90; }
    if name.contains("redis") || name.contains("valkey") || name.contains("keydb") { return 85; }
    if name.contains("mysql") || name.contains("mariadb") || name.contains("postgres") || name.contains("pg") || name.contains("mongo") { return 80; }
    if name.contains("elasticsearch") || name.contains("kibana") || name.contains("logstash") { return 75; }
    if name.contains("kafka") || name.contains("zookeeper") || name.contains("rabbitmq") { return 70; }
    if name.contains("docker") || name.contains("containerd") { return 65; }
    if name.contains("node") || name.contains("npm") || name.contains("yarn") { return 60; }
    if name.contains("python") || name.contains("gunicorn") || name.contains("uvicorn") { return 55; }
    if name.contains("sshd") || name.contains("cron") || name.contains("systemd") || name.contains("init") || name.contains("bash") || name.contains("getty") { return 10; }
    50
}

pub fn is_virt(n: &str) -> bool { n=="lo" || n.starts_with("docker") || n.starts_with("veth") || n.starts_with("br-") || n.starts_with("virbr") || n.starts_with("tap") || n.starts_with("tun") || n.starts_with("cali") || n.starts_with("flannel") || n.starts_with("kube") || n.starts_with("cni") }
fn fmem(kb: u64) -> String { if kb > 1048576 { format!("{:.2}GB", kb as f64 / 1048576.0) } else if kb > 1024 { format!("{:.0}MB", kb as f64 / 1024.0) } else { format!("{}KB", kb) } }
fn fmt_kb_short(kb: u64) -> String { fmem(kb) }

fn get_user(uid: u32) -> String {
    let passwd = r("/etc/passwd");
    for l in passwd.lines() {
        let parts: Vec<&str> = l.split(':').collect();
        if parts.len() >= 3 && parts[2].parse::<u32>().unwrap_or(0) == uid {
            return parts[0].to_string();
        }
    }
    uid.to_string()
}

fn get_service_name(pid: u32, cmd: &str) -> String {
    // Try systemd: systemctl status PID
    if let Ok(o) = std::process::Command::new("systemctl").args(["status", &format!("{}.service", pid)]).output() {
        let text = String::from_utf8_lossy(&o.stdout);
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("Loaded:") {
                // Extract service name from "loaded (/lib/systemd/system/xxx.service; ...)"
                if let Some(start) = line.rfind('/') {
                    if let Some(end) = line[start..].find(".service") {
                        return line[start+1..start+end].to_string();
                    }
                }
            }
        }
    }
    // Try reading /proc/PID/cgroup for systemd slice
    let cgroup = r(&format!("/proc/{}/cgroup", pid));
    for line in cgroup.lines() {
        if let Some(idx) = line.find(".service") {
            let before = &line[..idx];
            if let Some(slash) = before.rfind('/') {
                return before[slash+1..].to_string();
            }
        }
    }
    // Fallback: use cmd name without path
    cmd.split('/').last().unwrap_or("-").split_whitespace().next().unwrap_or("-").to_string()
}

fn scan_journal_errors() -> Vec<SysError> {
    let mut errors = Vec::new();
    // Get recent errors from journalctl (last 5 minutes, priority err and above)
    let output = std::process::Command::new("journalctl")
        .args(["-p","err","--since","5min ago","--no-pager","-n","30"])
        .output();
    if let Ok(o) = output {
        let text = String::from_utf8_lossy(&o.stdout);
        for line in text.lines() {
            if line.trim().is_empty()||line.contains("No entries")||line.contains("Boot "){continue;}
            // Parse: "May 27 10:30:15 hostname service[pid]: message"
            let parts: Vec<&str> = line.splitn(6, ' ').collect();
            let ts = if parts.len() >= 3 { format!("{} {} {}", parts[0], parts[1], parts[2]) } else { String::new() };
            let rest = parts.get(5).copied().unwrap_or(line);
            // Extract service name and pid from "service[pid]:"
            let (service, pid_str) = if let Some(bracket_start) = rest.find('[') {
                let svc = rest[..bracket_start].rsplit(' ').next().unwrap_or("?").to_string();
                let pid_s = if let Some(bracket_end) = rest.find(']') {
                    rest[bracket_start+1..bracket_end].to_string()
                } else { String::new() };
                (svc, pid_s)
            } else {
                let svc = rest.split(':').next().unwrap_or("?").trim().to_string();
                (svc, String::new())
            };
            // Extract message after ": "
            let msg = if let Some(colon) = rest.find(": ") { rest[colon+2..].to_string() } else { rest.to_string() };
            // Try to find executable path from pid
            let path = if !pid_str.is_empty() {
                std::fs::read_link(format!("/proc/{}/exe", pid_str)).ok()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| {
                        // Fallback: read cmdline
                        std::fs::read_to_string(format!("/proc/{}/cmdline", pid_str))
                            .ok().map(|c| c.replace('\0', " ").trim().to_string())
                            .unwrap_or_default()
                    })
            } else { String::new() };
            let sev = if msg.to_lowercase().contains("critical")||msg.to_lowercase().contains("fatal")||msg.to_lowercase().contains("panic")||msg.to_lowercase().contains("out of memory")||msg.to_lowercase().contains("oom-killer")||msg.to_lowercase().contains("no space") {
                ErrorSeverity::Critical
            } else if msg.to_lowercase().contains("error")||msg.to_lowercase().contains("fail")||msg.to_lowercase().contains("refused")||msg.to_lowercase().contains("timeout")||msg.to_lowercase().contains("denied")||msg.to_lowercase().contains("segfault")||msg.to_lowercase().contains("killed") {
                ErrorSeverity::Error
            } else { ErrorSeverity::Warning };
            errors.push(SysError { service, pid: pid_str, path, message: msg, severity: sev, timestamp: ts });
        }
    }
    // Also check specific service logs: elasticsearch, nginx, mysql, redis, docker, kafka
    let services = ["elasticsearch","nginx","mysql","mariadb","redis","docker","kafka","sshd","cron"];
    for svc in &services {
        if let Ok(o) = std::process::Command::new("journalctl").args(["-u",svc,"-p","err","--since","10min ago","--no-pager","-n","5"]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                if line.trim().is_empty()||line.contains("No entries")||line.contains("Boot "){continue;}
                let sev = if line.to_lowercase().contains("no space")||line.to_lowercase().contains("disk full")||line.to_lowercase().contains("out of memory") { ErrorSeverity::Critical }
                else { ErrorSeverity::Error };
                let short_msg = if line.len()>120{format!("{}...",&line[..117])}else{line.to_string()};
                errors.push(SysError{service:svc.to_string(),pid:String::new(),path:String::new(),message:short_msg,severity:sev,timestamp:String::new()});
            }
        }
    }
    // Deduplicate by message prefix
    errors.sort_by(|a,b| b.severity.partial_cmp(&a.severity).unwrap_or(std::cmp::Ordering::Equal));
    errors.dedup_by(|a,b| a.message[..a.message.len().min(30)] == b.message[..b.message.len().min(30)]);
    errors.truncate(20);
    errors
}

impl PartialOrd for ErrorSeverity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let v = |s: &ErrorSeverity| match s { ErrorSeverity::Critical=>3, ErrorSeverity::Error=>2, ErrorSeverity::Warning=>1 };
        v(self).partial_cmp(&v(other))
    }
}

fn scan_apps(ports: &[PortInfo]) -> Vec<AppInfo> {
    use std::collections::HashSet;
    let mut seen: HashSet<u32> = HashSet::new();
    let mut apps = Vec::new();
    if let Ok(dir) = fs::read_dir("/proc") {
        for e in dir.filter_map(|x| x.ok()) {
            let ns = e.file_name().to_string_lossy().into_owned();
            if !ns.chars().all(|c| c.is_ascii_digit()) { continue; }
            let pid: u32 = ns.parse().unwrap_or(0); if pid == 0 { continue; }
            if seen.contains(&pid) { continue; }
            let pd = format!("/proc/{}", ns);
            let stat = r(&format!("{}/stat", pd)); if stat.is_empty() { continue; }
            let cmd = if let Some(s) = stat.find('(') { if let Some(e) = stat.rfind(')') { stat[s+1..e].to_string() } else { String::new() } } else { String::new() };
            let after: Vec<&str> = stat.splitn(2,')').nth(1).unwrap_or("").split_whitespace().collect();
            if after.len() < 13 { continue; }
            let ut: u64 = after[11].parse().unwrap_or(0); let st: u64 = after[12].parse().unwrap_or(0);
            let mem: u64 = r(&format!("{}/status", pd)).lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1).unwrap_or("0").parse().ok()).unwrap_or(0);
            // Get full cmdline for better detection
            let cmdline = r(&format!("{}/cmdline", pd)).replace('\0', " ");
            let full_cmd = if cmdline.trim().is_empty() { cmd.clone() } else { cmdline.trim().to_string() };
            // Find ALL listening ports for this PID
            let pid_ports: Vec<u16> = ports.iter().filter(|p| p.pid == pid).map(|p| p.port).collect();
            // Categorize
            let cat = categorize(&cmd, &full_cmd);
            if cat == AppCategory::Other { continue; } // Skip system processes
            seen.insert(pid);
            let uid = after[2].parse::<u32>().unwrap_or(0);
            let user_name = get_user(uid);
            let threads2: u32 = after.get(17).and_then(|s| s.parse().ok()).unwrap_or(0);
            let start_time = after.get(19).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
            let total_cpu = (ut+st) as f64;
            let cpu_pct_val = if start_time > 0 { total_cpu / ((start_time as f64).max(1.0)) * 100.0 } else { 0.0 };
            let mem_pct_val = 0.0; // Will be updated in tick
            // Try to get systemd service name
            let service_name = get_service_name(pid, &cmd);
            apps.push(AppInfo { name: full_cmd, pid, ports: pid_ports, cpu: total_cpu, mem_kb: mem, category: cat, running: true, user: user_name, threads: threads2, cpu_pct: cpu_pct_val, mem_pct: mem_pct_val, service_name });
        }
    }
    apps.retain(|a| !matches!(a.category, AppCategory::Container));
    apps.sort_by(|a,b| category_order(&a.category).cmp(&category_order(&b.category)).then(b.ports.first().unwrap_or(&0).cmp(a.ports.first().unwrap_or(&0))));
    apps.truncate(20);
    apps
}

fn categorize(name: &str, full_cmd: &str) -> AppCategory {
    let s = format!("{} {}", name.to_lowercase(), full_cmd.to_lowercase());
    if s.contains("java") || s.contains("tomcat") || s.contains("jetty") || s.contains("spring") || s.contains("jar") || s.contains("jdk") { return AppCategory::Java; }
    if s.contains("nginx") || s.contains("apache") || s.contains("httpd") || s.contains("caddy") || s.contains("traefik") || s.contains("haproxy") { return AppCategory::WebServer; }
    if s.contains("mysqld") || s.contains("mariadb") || s.contains("postgres") || s.contains("mongod") || s.contains("clickhouse") { return AppCategory::Database; }
    if s.contains("redis") || s.contains("valkey") || s.contains("memcached") || s.contains("keydb") { return AppCategory::Cache; }
    if s.contains("kafka") || s.contains("rabbitmq") || s.contains("pulsar") || s.contains("nats") { return AppCategory::MessageQueue; }
    if s.contains("elasticsearch") || s.contains("kibana") || s.contains("logstash") || s.contains("solr") { return AppCategory::Search; }
    if s.contains("docker") || s.contains("containerd") || s.contains("k3s") || s.contains("kube") { return AppCategory::Container; }
    AppCategory::Other
}

fn category_order(c: &AppCategory) -> u8 {
    match c { AppCategory::Java=>1, AppCategory::WebServer=>2, AppCategory::Database=>3, AppCategory::Cache=>4, AppCategory::Search=>5, AppCategory::MessageQueue=>6, AppCategory::Container=>7, AppCategory::Other=>8 }
}

#[cfg(test)]
mod test {
    #[test] fn test_gather_data() {
        let d = super::Dashboard::new();
        let data = &d.data;
        println!("top_cpu count: {}", data.top_cpu.len());
        println!("top_mem count: {}", data.top_mem.len());
        println!("ips: {:?}", data.ips);
        println!("disks: {:?}", data.disks);
        println!("apps: {}", data.apps.len());
        println!("sys_errors: {}", data.sys_errors.len());
        println!("mem_total: {}MB, mem_used: {}MB", data.mem_total, data.mem_used);
        println!("cpu_pct: {:.1}%, cpu_cores: {}", data.cpu_pct, data.cpu_cores);
        assert!(!data.ips.is_empty(), "IPs should not be empty! hostname -I or ip addr should return data");
        assert!(data.mem_total > 0, "Memory total should be > 0");
    }
}
