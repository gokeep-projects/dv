// Docker Manager: containers, images, logs, start/stop/restart
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use std::process::Command;

#[derive(Clone)]
pub struct DockerState {
    pub containers: Vec<Container>, pub images: Vec<Image>,
    pub selected: usize, pub mode: DockerMode, pub output: String,
    pub logs: Vec<String>, pub scroll: usize, pub loaded: bool,
}

#[derive(Clone)]
pub struct Container {
    pub id: String, pub name: String, pub image: String,
    pub status: String, pub ports: String, pub cpu: String, pub mem: String,
}

#[derive(Clone)]
pub struct Image { pub repo: String, pub tag: String, pub id: String, pub size: String }

#[derive(Clone, Copy, PartialEq)]
pub enum DockerMode { Normal, Inspect, Logs, Images }

impl Default for DockerState {
    fn default() -> Self {
        Self { containers: vec![], images: vec![], selected: 0, mode: DockerMode::Normal,
            output: String::new(), logs: vec![], scroll: 0, loaded: false }
    }
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

impl DockerState {
    pub fn current(&self) -> Option<&Container> { self.containers.get(self.selected) }
    pub fn fetch_all(&mut self) { self.fetch_containers(); self.fetch_images(); self.loaded = true; }

    pub fn fetch_containers(&mut self) {
        let (out, _) = docker(&["ps", "-a", "--format", "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}"]);
        self.containers.clear();
        // Batch stats: one call for all containers
        let (stats_out, _) = docker(&["stats", "--no-stream", "--format", "{{.Name}}\t{{.CPUPerc}}\t{{.MemPerc}}"]);
        let stats_map: std::collections::HashMap<String,(String,String)> = stats_out.lines()
            .filter_map(|l|{let p:Vec<&str>=l.split('\t').collect();if p.len()>=3{Some((p[0].to_string(),(p[1].to_string(),p[2].to_string())))}else{None}})
            .collect();
        for line in out.lines() {
            let p: Vec<&str> = line.split('\t').collect();
            if p.len() < 4 { continue; }
            let name = p.get(1).unwrap_or(&"-").to_string();
            let (cpu, mem) = stats_map.get(&name).cloned().unwrap_or_else(|| ("-".into(), "-".into()));
            self.containers.push(Container {
                id: p[0].to_string(), name,
                image: p.get(2).unwrap_or(&"-").to_string(),
                status: p.get(3).unwrap_or(&"-").to_string(),
                ports: p.get(4).unwrap_or(&"").to_string(), cpu, mem,
            });
        }
    }

    pub fn fetch_images(&mut self) {
        let (out, _) = docker(&["images", "--format", "{{.Repository}}\t{{.Tag}}\t{{.ID}}\t{{.Size}}"]);
        self.images.clear();
        for line in out.lines() {
            let p: Vec<&str> = line.split('\t').collect();
            if p.len() < 4 { continue; }
            self.images.push(Image { repo: p[0].to_string(), tag: p[1].to_string(), id: p[2].to_string(), size: p[3].to_string() });
        }
    }

    pub fn inspect(&mut self) {
        if let Some(c) = self.current() {
            let (out, _) = docker(&["inspect", &c.id]);
            self.output = out; self.mode = DockerMode::Inspect;
        }
    }

    pub fn fetch_logs(&mut self) {
        if let Some(c) = self.current() {
            let (out, _) = docker(&["logs", "--tail", "100", &c.id]);
            self.logs = out.lines().map(|l| l.to_string()).collect();
            self.mode = DockerMode::Logs;
        }
    }

    pub fn start(&mut self) {
        if let Some(c) = self.current() {
            let id = c.id.clone();
            let (out, err) = docker(&["start", &id]);
            self.output = if err { format!("✗ {}", out) } else { format!("✓ {}", id) };
            self.fetch_containers();
        }
    }
    pub fn stop(&mut self) {
        if let Some(c) = self.current() {
            let id = c.id.clone();
            let (out, err) = docker(&["stop", &id]);
            self.output = if err { format!("✗ {}", out) } else { format!("✓ {}", id) };
            self.fetch_containers();
        }
    }
    pub fn restart(&mut self) {
        if let Some(c) = self.current() {
            let id = c.id.clone();
            let (out, err) = docker(&["restart", &id]);
            self.output = if err { format!("✗ {}", out) } else { format!("✓ {}", id) };
            self.fetch_containers();
        }
    }
    pub fn rm(&mut self) {
        if let Some(c) = self.current() {
            let id = c.id.clone();
            let (out, err) = docker(&["rm", "-f", &id]);
            self.output = if err { format!("✗ {}", out) } else { format!("✓ 已删除 {}", id) };
            self.selected = 0; self.fetch_containers(); self.mode = DockerMode::Normal;
        }
    }
}

fn dm(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.text_dim)) }
fn dp(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.primary).bold()) }
fn dt(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.text)) }
fn da(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.accent).bold()) }

pub fn render_docker_overview(f: &mut Frame, area: Rect, state: &mut DockerState, t: &Theme) {
    let [top, bot] = Layout::vertical([Constraint::Percentage(65), Constraint::Percentage(35)]).areas(area);

    // Containers - wide full-width table
    let cb = Block::default().title(dp(t, &format!(" 容器 ({}个) ↑↓选择 s启动 p停止 t重启 x删除 i详情 l日志 ", state.containers.len())))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.primary)).style(Style::default().bg(t.surface));
    f.render_widget(cb.clone(), top);
    let ci = cb.inner(top).inner(Margin::new(1, 0));
    if state.containers.is_empty() {
        f.render_widget(Paragraph::new("  没有容器\n  r 刷新").centered(), ci);
    } else {
        let hdr = vec![
            dm(t, " CONTAINER ID   "), dm(t, "NAME              "), dm(t, "IMAGE               "),
            dm(t, "STATUS              "), dm(t, "CPU   "), dm(t, "MEM   "), dm(t, "PORTS                      "),
        ];
        let sep = dm(t, &"\u{2500}".repeat(area.width as usize - 4));
        let mut lines = vec![Line::from(hdr), Line::from(sep)];
        for (i, c) in state.containers.iter().skip(state.scroll).take(10).enumerate() {
            let sel = i + state.scroll == state.selected;
            let sty = if sel { Style::default().fg(t.primary).bg(t.surface_alt).bold() } else { Style::default().fg(t.text) };
            let sc = if c.status.contains("Up") { t.success } else { t.error };
            let si = if c.status.contains("Up") { "\u{25cf}" } else { "\u{25cb}" };
            lines.push(Line::from(vec![
                Span::styled(format!(" {:<15}", ts(&c.id, 15)), sty.clone()),
                Span::styled(format!(" {:<18}", ts(&c.name, 18)), sty.clone()),
                Span::styled(format!(" {:<20}", ts(&c.image, 20)), sty.clone()),
                Span::styled(format!("{}{}", si, ts(&c.status, 18)), Style::default().fg(sc)),
                Span::styled(format!(" {:<6}", c.cpu), sty.clone()),
                Span::styled(format!(" {:<6}", c.mem), sty.clone()),
                Span::styled(ts(&c.ports, 28), Style::default().fg(t.text_dim)),
            ]));
        }
        f.render_widget(Paragraph::new(lines), ci);
    }

    // Bottom: images + selected container detail
    let [img_a, detail_a] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(bot);

    // Images
    let ib = Block::default().title(dp(t, &format!(" 镜像 ({}个) ", state.images.len())))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(ib.clone(), img_a);
    let ii = ib.inner(img_a).inner(Margin::new(1, 0));
    if state.images.is_empty() {
        f.render_widget(Paragraph::new("  无镜像").centered(), ii);
    } else {
        let mut lines = vec![
            Line::from(vec![dm(t, " REPOSITORY            "), dm(t, "TAG       "), dm(t, "SIZE      ")]),
            Line::from(dm(t, "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}")),
        ];
        for img in state.images.iter().take(8) {
            lines.push(Line::from(vec![
                dt(t, &format!(" {:<21}", ts(&img.repo, 20))),
                dt(t, &format!(" {:<9}", ts(&img.tag, 9))),
                da(t, &format!(" {}", img.size)),
            ]));
        }
        f.render_widget(Paragraph::new(lines), ii);
    }

    // Selected container detail
    let db = Block::default().title(dp(t, " 选中容器详情 "))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.accent)).style(Style::default().bg(t.surface));
    f.render_widget(db.clone(), detail_a);
    let di = db.inner(detail_a).inner(Margin::new(1, 1));
    if let Some(c) = state.current() {
        let st_c = if c.status.contains("Up") { t.success } else { t.error };
        let lines = vec![
            Line::from(vec![dm(t, "ID:     "), dt(t, &c.id)]),
            Line::from(vec![dm(t, "Name:   "), dt(t, &c.name)]),
            Line::from(vec![dm(t, "Image:  "), dt(t, &c.image)]),
            Line::from(vec![dm(t, "Status: "), Span::styled(&c.status, Style::default().fg(st_c).bold())]),
            Line::from(vec![dm(t, "CPU:    "), dt(t, &c.cpu), dm(t, "   MEM: "), dt(t, &c.mem)]),
            Line::from(vec![dm(t, "Ports:  "), dt(t, if c.ports.is_empty(){"-"}else{&c.ports})]),
            Line::from(dm(t, "")),
            Line::from(dm(t, "s:启动 p:停止 t:重启 x:删除 i:inspect l:日志")),
        ];
        f.render_widget(Paragraph::new(lines), di);
    } else {
        f.render_widget(Paragraph::new(dm(t, "  ↑↓ 选择容器查看详情")).centered(), di);
    }
}

pub fn render_docker_inspect(f: &mut Frame, area: Rect, state: &DockerState, t: &Theme) {
    let b = Block::default().title(dp(t, " docker inspect ")).borders(Borders::ALL)
        .border_type(BorderType::Rounded).border_style(Style::default().fg(t.primary))
        .style(Style::default().bg(t.surface));
    f.render_widget(b.clone(), area);
    let i = b.inner(area).inner(Margin::new(1, 0));
    let v: serde_json::Value = serde_json::from_str(&state.output).unwrap_or_default();
    let mut lines = vec![];
    if let Some(arr) = v.as_array().and_then(|a| a.first()) {
        let name = arr.get("Name").and_then(|v| v.as_str()).unwrap_or("-").trim_start_matches('/');
        let st = arr.get("State").and_then(|v| v.get("Status")).and_then(|v| v.as_str()).unwrap_or("-");
        let running = arr.get("State").and_then(|v| v.get("Running")).and_then(|v| v.as_bool()).unwrap_or(false);
        let sc = if running { t.success } else { t.error };
        let ip = arr.get("NetworkSettings").and_then(|v| v.get("IPAddress")).and_then(|v| v.as_str()).unwrap_or("-");
        let img = arr.get("Config").and_then(|v| v.get("Image")).and_then(|v| v.as_str()).unwrap_or("-");
        let cmd_v = arr.get("Config").and_then(|v| v.get("Cmd")).map(|v| v.to_string()).unwrap_or_default();
        lines.push(Line::from(dp(t, &format!(" ◆ {} ", name))));
        lines.push(Line::from(dm(t, "──────────────────────────────────────────────")));
        lines.push(Line::from(vec![dm(t, "状态: "), Span::styled(st, Style::default().fg(sc).bold())]));
        lines.push(Line::from(vec![dm(t, "镜像: "), dt(t, img), dm(t, "  IP: "), dt(t, ip)]));
        lines.push(Line::from(vec![dm(t, "Cmd:  "), dt(t, &ts(&cmd_v, 60))]));
        // Ports
        if let Some(ports) = arr.get("NetworkSettings").and_then(|v| v.get("Ports")).and_then(|v| v.as_object()) {
            lines.push(Line::from(dp(t, " 端口:")));
            for (cp, bindings) in ports {
                if let Some(b) = bindings.as_array().and_then(|a| a.first()) {
                    let hp = b.get("HostPort").and_then(|v| v.as_str()).unwrap_or("-");
                    lines.push(Line::from(vec![dm(t, "  "), dt(t, &format!("{}:{}", hp, cp))]));
                }
            }
        }
    } else {
        lines.push(Line::from(dt(t, &ts(&state.output, 200))));
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), i);
}

pub fn render_docker_logs(f: &mut Frame, area: Rect, state: &DockerState, t: &Theme) {
    let b = Block::default().title(dp(t, " 容器日志 (tail 100) ")).borders(Borders::ALL)
        .border_type(BorderType::Rounded).border_style(Style::default().fg(t.warning))
        .style(Style::default().bg(t.surface));
    f.render_widget(b.clone(), area);
    let i = b.inner(area).inner(Margin::new(1, 0));
    let lines: Vec<Line> = state.logs.iter().skip(state.scroll).take(28).map(|l| {
        let c = if l.contains("ERROR") || l.contains("error") || l.contains("FATAL") { t.error }
        else if l.contains("WARN") || l.contains("warn") { t.warning }
        else { t.text_dim };
        Line::from(Span::styled(ts(l, 140), Style::default().fg(c)))
    }).collect();
    f.render_widget(Paragraph::new(lines), i);
}

fn ts(s: &str, n: usize) -> String {let cs:Vec<char>=s.chars().collect();if cs.len()<=n{s.into()}else{format!("{}…",cs.iter().take(n-1).collect::<String>())}}

