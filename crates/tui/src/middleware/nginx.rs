// Nginx: status check, config viewer, log access, process monitoring
// Callers: middleware/mod.rs line 5, app.rs for rendering
// No direct file I/O; persistence via config.rs

use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use super::config::{load_config, add_nginx_conn, NginxConn};
use std::process::Command;

#[derive(Clone)]
pub struct NginxState {
    pub connections: Vec<NginxConn>, pub selected: usize, pub mode: NginxMode,
    pub output: String, pub config: String, pub logs: Vec<String>,
    pub log_path: String, pub pid_info: String, pub scroll: usize,
    pub config_name: String, pub config_path: String, pub log_conf: String,
    pub edit_field: u8,
}

#[derive(Clone, Copy, PartialEq)]
pub enum NginxMode { Normal, AddConn, ViewConfig, Logs }

impl Default for NginxState {
    fn default() -> Self {
        Self {
            connections: vec![], selected: 0, mode: NginxMode::Normal, output: String::new(),
            config: String::new(), logs: vec![], log_path: String::new(),
            pid_info: String::new(), scroll: 0, config_name: String::new(),
            config_path: "/etc/nginx/nginx.conf".into(), log_conf: "/var/log/nginx/".into(),
            edit_field: 0,
        }
    }
}

impl NginxState {
    pub fn refresh_conns(&mut self) { self.connections = load_config().nginx; }
    pub fn current(&self) -> Option<&NginxConn> { self.connections.get(self.selected) }

    pub fn fetch_info(&mut self) {
        self.check_process(); self.find_logs();
        let ci = if self.config.is_empty() { "未加载" } else { "已加载" };
        let li = if self.logs.is_empty() { "未找到" } else { &format!("{}条", self.logs.len()) };
        self.output = format!("Nginx状态\n进程: {}\n配置: {}\n日志: {}", self.pid_info, ci, li);
    }

    fn check_process(&mut self) {
        if let Ok(out) = Command::new("pgrep").arg("-x").arg("nginx").output() {
        let out_s = String::from_utf8_lossy(&out.stdout).to_string();
            let pids: Vec<&str> = out_s.lines().filter(|l| !l.is_empty()).collect();
            self.pid_info = if pids.is_empty() { "未运行".into() }
            else { format!("运行中 ({}进程: {})", pids.len(), pids.join(",")) };
        } else if let Ok(out) = Command::new("sh").arg("-c")
            .arg("ps aux|grep -E 'nginx.*master|nginx.*worker'|grep -v grep").output()
        {
            let c = String::from_utf8_lossy(&out.stdout).lines().filter(|l| !l.is_empty()).count();
            self.pid_info = if c > 0 { format!("运行中 ({}进程)", c) } else { "未运行".into() };
        } else { self.pid_info = "未知".into(); }
    }

    pub fn load_config(&mut self) {
        let path = self.current().and_then(|c| c.config_path.as_deref()).unwrap_or("/etc/nginx/nginx.conf");
        self.config = std::fs::read_to_string(path).unwrap_or_else(|e| format!("读取失败: {}", e));
    }

    pub fn find_logs(&mut self) {
        let candidates = ["/var/log/nginx/error.log", "/var/log/nginx/access.log",
            "/usr/local/nginx/logs/error.log"];
        for p in candidates { if std::path::Path::new(p).exists() { self.log_path = p.into(); break; } }
        if self.log_path.is_empty() && std::path::Path::new("/var/log/nginx/").is_dir() {
            self.log_path = "/var/log/nginx/error.log".into();
        }
        self.tail_logs();
    }

    pub fn tail_logs(&mut self) {
        if self.log_path.is_empty() { return; }
        if let Ok(out) = Command::new("tail").arg("-n").arg("50").arg(&self.log_path).output() {
            self.logs = String::from_utf8_lossy(&out.stdout).lines().map(|l| l.to_string()).collect();
        }
    }

    pub fn test_config(&mut self) {
        let path = self.current().and_then(|c| c.config_path.as_deref()).unwrap_or("/etc/nginx/nginx.conf");
        if let Ok(out) = Command::new("nginx").arg("-t").arg("-c").arg(path).output() {
            self.output = String::from_utf8_lossy(&out.stderr).to_string();
            if self.output.is_empty() { self.output = String::from_utf8_lossy(&out.stdout).to_string(); }
            if self.output.is_empty() { self.output = "配置测试通过 ✓".into(); }
        } else { self.output = "nginx命令不可用".into(); }
    }

    pub fn connect_new(&mut self) {
        let conn = NginxConn {
            name: if self.config_name.is_empty() { "nginx".into() } else { self.config_name.clone() },
            config_path: if self.config_path.is_empty() { None } else { Some(self.config_path.clone()) },
            log_path: if self.log_conf.is_empty() { None } else { Some(self.log_conf.clone()) },
            pid_path: None,
        };
        let name = conn.name.clone(); add_nginx_conn(conn);
        self.refresh_conns(); self.selected = self.connections.iter().position(|c| c.name == name).unwrap_or(0);
        self.mode = NginxMode::Normal; self.fetch_info();
    }
}

fn dm(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.text_dim)) }
fn dp(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.primary).bold()) }
fn dt(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.text)) }

pub fn render_nginx_overview(f: &mut Frame, area: Rect, state: &mut NginxState, t: &Theme) {
    let [sidebar, main] = Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)]).areas(area);
    let sb = Block::default().title(dp(t, "Nginx")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(sb.clone(), sidebar);
    let si = sb.inner(sidebar).inner(Margin::new(1, 0));
    let items: Vec<ListItem> = state.connections.iter().enumerate().map(|(i, c)| {
        let s = if i == state.selected { Style::default().fg(t.primary).bg(t.surface_alt).bold() } else { Style::default().fg(t.text) };
        ListItem::new(format!(" {} {}", if i == state.selected { "▶" } else { " " }, c.name)).style(s)
    }).collect();
    let mut ls = ListState::default(); ls.select(Some(state.selected));
    f.render_stateful_widget(List::new(items), si, &mut ls);

    let mb = Block::default().title(dp(t, "Nginx 状态")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(mb.clone(), main);
    let mi = mb.inner(main).inner(Margin::new(1, 1));
    let mut lines = vec![];
    if let Some(c) = state.current() {
        lines.push(Line::from(dp(t, &format!("◉ {}", c.name))));
        lines.push(Line::from(dm(t, "──────────────────────────────────────────────")));
        for l in state.output.lines() { lines.push(Line::from(dt(t, l))); }
        lines.push(Line::from(""));
        lines.push(Line::from(dm(t, "c:配置  t:测试配置  l:日志  a:添加  Enter:刷新")));
    } else {
        lines.push(Line::from("  没有Nginx配置\n  a: 添加新配置"));
    }
    f.render_widget(Paragraph::new(lines), mi);
}

pub fn render_nginx_config(f: &mut Frame, area: Rect, state: &NginxState, t: &Theme) {
    let b = Block::default().title(dp(t, "Nginx 配置")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(), area);
    let i = b.inner(area).inner(Margin::new(1, 0));
    let lines: Vec<Line> = state.config.lines().skip(state.scroll).take(40).map(|l| Line::from(dt(t, l))).collect();
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), i);
}

pub fn render_nginx_logs(f: &mut Frame, area: Rect, state: &NginxState, t: &Theme) {
    let b = Block::default().title(dp(t, &format!("Nginx 日志 ({})", ts(&state.log_path, 40))))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.warning)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(), area);
    let i = b.inner(area).inner(Margin::new(1, 0));
    let lines: Vec<Line> = state.logs.iter().skip(state.scroll).take(35).map(|l| {
        let c = if l.contains("error") || l.contains("Error") { t.error }
        else if l.contains("warn") { t.warning } else { t.text_dim };
        Line::from(Span::styled(ts(l, 120), Style::default().fg(c)))
    }).collect();
    f.render_widget(Paragraph::new(lines), i);
}

pub fn render_nginx_add(f: &mut Frame, area: Rect, state: &mut NginxState, t: &Theme) {
    let b = Block::default().title(dp(t, "添加 Nginx")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.primary)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(), area);
    let i = b.inner(area).inner(Margin::new(2, 2));
    let fields: [(&str, &str, u8); 3] = [
        ("名称", &state.config_name, 0), ("配置文件", &state.config_path, 1), ("日志路径", &state.log_conf, 2),
    ];
    let lines: Vec<Line> = fields.iter().map(|(label, val, fi)| {
        let (pre, post) = if state.edit_field == *fi { ("▶ ", " ▍") } else { ("  ", "") };
        Line::from(vec![
            Span::styled(format!("{}{:<10}", pre, label), if state.edit_field == *fi { Style::default().fg(t.primary).bold() } else { Style::default().fg(t.text_dim) }),
            Span::styled(format!("{}{}", val, post), Style::default().fg(t.text)),
        ])
    }).collect();
    f.render_widget(Paragraph::new(lines), i);
}

fn ts(s: &str, n: usize) -> String {let cs:Vec<char>=s.chars().collect();if cs.len()<=n{s.into()}else{format!("{}…",cs.iter().take(n-1).collect::<String>())}}
