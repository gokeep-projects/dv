use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;

struct SysinfoPlugin;

impl Plugin for SysinfoPlugin {
    fn metadata(&self) -> PluginMetadata { PluginMetadata {
        name: "sysinfo".into(), version: "0.1.0".into(),
        description: "System dashboard: OS, CPU, memory, disk, network, resources".into(),
        author: "DevTool Team".into(), category: PluginCategory::SystemTool,
        actions: vec![
            PluginAction { name: "dashboard".into(), description: "Full system status dashboard".into(), params: vec![] },
            PluginAction { name: "cpu".into(), description: "CPU info and load average".into(), params: vec![] },
            PluginAction { name: "memory".into(), description: "Memory usage details".into(), params: vec![] },
            PluginAction { name: "disk".into(), description: "Disk partitions and usage".into(), params: vec![] },
            PluginAction { name: "network".into(), description: "Network interfaces, IPs, listening ports".into(), params: vec![] },
        ],
    }}

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        match input.action.as_str() {
            "dashboard" => Ok(PluginOutput{success:true, data:Self::dashboard(), error:None, metadata:None}),
            "cpu" => Ok(PluginOutput{success:true, data:Self::cpu(), error:None, metadata:None}),
            "memory" => Ok(PluginOutput{success:true, data:Self::mem(), error:None, metadata:None}),
            "disk" => Ok(PluginOutput{success:true, data:Self::disk(), error:None, metadata:None}),
            "network" => Ok(PluginOutput{success:true, data:Self::net(), error:None, metadata:None}),
            _ => Err(PluginError::InvalidAction(input.action)),
        }
    }
    fn tui_view(&self) -> Option<TuiViewDef> { Some(TuiViewDef { title: "System Dashboard".into(), component_type: TuiComponentType::Table }) }
    fn web_handlers(&self) -> Vec<WebHandlerDef> { vec![] }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::fs;
    pub fn read(p: &str) -> String { fs::read_to_string(p).unwrap_or_default() }

    pub fn dashboard() -> String {
        format!("══════════════════════════════════════\n         SYSTEM DASHBOARD\n══════════════════════════════════════\n\n【操作系统】\n{}\n【CPU】\n{}\n【内存】\n{}\n【磁盘】\n{}\n【网络】\n{}\n【系统资源】\n{}",
            os(), cpu(), mem(), disk(), net(), res())
    }
    pub fn os() -> String {
        let rel = read("/etc/os-release");
        let mut name: String = "Linux".into(); let mut ver = String::new();
        for l in rel.lines() { if l.starts_with("PRETTY_NAME=") { name = l.trim_start_matches("PRETTY_NAME=").trim_matches('"').into(); } if l.starts_with("VERSION_ID=") { ver = l.trim_start_matches("VERSION_ID=").trim_matches('"').into(); } }
        let k = read("/proc/version").split_whitespace().take(3).collect::<Vec<_>>().join(" ");
        let hn = read("/proc/sys/kernel/hostname").trim().to_string();
        let up: f64 = read("/proc/uptime").split_whitespace().next().unwrap_or("0").parse().unwrap_or(0.0);
        let (d,h,m) = (up as u64/86400, (up as u64%86400)/3600, (up as u64%3600)/60);
        format!("  系统: {}\n  版本: {}\n  内核: {}\n  主机: {}\n  运行: {}天{}时{}分\n", name, ver, k, hn, d, h, m)
    }
    pub fn cpu() -> String {
        let mut model="?".to_string(); let mut cores=0u32; let mut mhz=0f64;
        for l in read("/proc/cpuinfo").lines() { if l.starts_with("model name")&&model=="?" { model=l.split(':').nth(1).unwrap_or("?").trim().into(); } if l.starts_with("processor") { cores+=1; } if l.starts_with("cpu MHz")&&mhz==0.0 { mhz=l.split(':').nth(1).unwrap_or("0").trim().parse().unwrap_or(0.0); } }
        let la = read("/proc/loadavg"); let ld:Vec<&str> = la.split_whitespace().take(3).collect();
        format!("  型号: {}\n  核心: {} cores @ {:.0} MHz\n  负载: {} (1m/5m/15m)\n", model, cores, mhz, if ld.len()==3{format!("{} / {} / {}",ld[0],ld[1],ld[2])}else{"?".into()})
    }
    pub fn mem() -> String {
        let mut t=0u64; let mut f=0u64; let mut a=0u64; let mut b=0u64; let mut c=0u64; let mut st=0u64; let mut sf=0u64;
        for l in read("/proc/meminfo").lines() { let v=l.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0); if l.starts_with("MemTotal:"){t=v;} if l.starts_with("MemFree:"){f=v;} if l.starts_with("MemAvailable:"){a=v;} if l.starts_with("Buffers:"){b=v;} if l.starts_with("Cached:"){c=v;} if l.starts_with("SwapTotal:"){st=v;} if l.starts_with("SwapFree:"){sf=v;} }
        let u=t.saturating_sub(f); let p=if t>0{u as f64/t as f64*100.0}else{0.0};
        format!("  总计: {} MB  已用: {} MB ({:.1}%)  可用: {} MB\n  Buff/Cache: {} MB  Swap: {} / {} MB\n", t/1024, u/1024, p, a/1024, (b+c)/1024, st.saturating_sub(sf)/1024, st/1024)
    }
    pub fn disk() -> String {
        let mut out = String::new();
        if let Ok(o) = std::process::Command::new("df").args(["-h","-x","tmpfs","-x","devtmpfs","-x","overlay","-x","squashfs"]).output() {
            let df = String::from_utf8_lossy(&o.stdout);
            for l in df.lines().skip(1) { let cols:Vec<&str>=l.split_whitespace().collect(); if cols.len()>=6 { out.push_str(&format!("  {: <18} {: >6} {: >5} {: >5} {: >5} {}\n",cols[0],cols[1],cols[2],cols[3],cols[4],cols.get(5).unwrap_or(&""))); } }
        }
        if out.is_empty(){out.push_str("  (无物理磁盘)\n");} out
    }
    pub fn net() -> String {
        let mut out=String::new();
        for l in read("/proc/net/dev").lines().skip(2) { let iface=l.split(':').next().unwrap_or("").trim(); if iface.is_empty()||iface=="lo"{continue;} let p:Vec<&str>=l.split_whitespace().collect(); let rx=p.get(1).unwrap_or(&"0"); let tx=p.get(9).unwrap_or(&"0"); let mac=read(&format!("/sys/class/net/{}/address",iface)).trim().to_string(); out.push_str(&format!("  {}: rx={} tx={} mac={}\n",iface,fb(rx),fb(tx),mac)); }
        if let Ok(o)=std::process::Command::new("ip").args(["-4","addr","show"]).output() { for l in String::from_utf8_lossy(&o.stdout).lines() { if l.trim().starts_with("inet "){out.push_str(&format!("  {}\n",l.trim()));} } }
        if let Ok(o)=std::process::Command::new("ss").args(["-tlnp"]).output() { let ss=String::from_utf8_lossy(&o.stdout); let ls:Vec<&str>=ss.lines().filter(|l|l.contains("LISTEN")).collect(); if !ls.is_empty() { out.push_str("\n  监听端口:\n"); for l in ls.iter().take(10){out.push_str(&format!("  {}\n",l.trim()));} } }
        if out.is_empty(){out.push_str("  (无网络信息)\n");} out
    }
    pub fn res() -> String {
        let mut out=String::new();
        if let Ok(e)=fs::read_dir("/proc/self/fd"){out.push_str(&format!("  文件描述符(当前进程): {}\n",e.count()));}
        for l in read("/proc/self/limits").lines(){if l.starts_with("Max open files"){out.push_str(&format!("  {}\n",l.trim()));}}
        if let Ok(e)=fs::read_dir("/proc"){let pc=e.filter_map(|x|x.ok()).filter(|x|x.file_type().map(|t|t.is_dir()).unwrap_or(false)&&x.file_name().to_string_lossy().chars().all(|c|c.is_ascii_digit())).count();out.push_str(&format!("  进程总数: {}\n",pc));}
        out.push_str(&format!("  kernel.pid_max: {}\n  fs.file-max: {}\n",read("/proc/sys/kernel/pid_max").trim(),read("/proc/sys/fs/file-max").trim())); out
    }
    pub fn fb(s: &str) -> String { let b:f64=s.parse().unwrap_or(0.0); if b>1e12{format!("{:.1}TB",b/1e12)}else if b>1e9{format!("{:.1}GB",b/1e9)}else if b>1e6{format!("{:.1}MB",b/1e6)}else if b>1e3{format!("{:.1}KB",b/1e3)}else{format!("{}B",b as u64)} }
}

#[cfg(not(target_os = "linux"))]
mod linux {
    pub fn dashboard()->String{"系统信息仅在 Linux 上可用\n".into()}
    pub fn os()->String{String::new()} pub fn cpu()->String{String::new()} pub fn mem()->String{String::new()}
    pub fn disk()->String{String::new()} pub fn net()->String{String::new()} pub fn res()->String{String::new()}
    pub fn fb(_:&str)->String{String::new()}
}

impl SysinfoPlugin {
    fn dashboard() -> String { linux::dashboard() }
    fn cpu() -> String { linux::cpu() }
    fn mem() -> String { linux::mem() }
    fn disk() -> String { linux::disk() }
    fn net() -> String { linux::net() }
}

#[no_mangle] pub extern "C" fn _plugin_create() -> Box<dyn Plugin> { Box::new(SysinfoPlugin) }
