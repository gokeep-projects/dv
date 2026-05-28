use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use super::config::{load_config, add_kafka_conn, KafkaConn};
use std::process::Command;

#[derive(Clone)]
pub struct KafkaState {
    pub connections: Vec<KafkaConn>, pub selected: usize, pub mode: KafkaMode,
    pub output: String, pub topics: Vec<(String,i32,i32)>, pub groups: Vec<(String,String)>,
    pub logs: Vec<String>, pub log_path: String, pub scroll: usize,
    pub config_name: String, pub config_brokers: String,
    pub config_user: String, pub config_pass: String, pub edit_field: u8,
}

#[derive(Clone, Copy, PartialEq)]
pub enum KafkaMode { Normal, AddConn, Topics, Groups, Logs }

impl Default for KafkaState {
    fn default() -> Self {
        Self {
            connections: vec![], selected: 0, mode: KafkaMode::Normal, output: String::new(),
            topics: vec![], groups: vec![], logs: vec![], log_path: String::new(),
            scroll: 0, config_name: String::new(), config_brokers: "localhost:9092".into(),
            config_user: String::new(), config_pass: String::new(), edit_field: 0,
        }
    }
}

fn run_kafka_cmd(brokers: &str, args: &[&str]) -> (String, bool) {
    let mut cmd = Command::new("kafka-topics.sh");
    cmd.arg("--bootstrap-server").arg(brokers);
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

fn run_kafka_groups(brokers: &str) -> (String, bool) {
    let mut cmd = Command::new("kafka-consumer-groups.sh");
    cmd.arg("--bootstrap-server").arg(brokers).arg("--list");
    match cmd.output() {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            let e = String::from_utf8_lossy(&o.stderr).to_string();
            (if s.is_empty() { e } else { s }, !o.status.success())
        }
        Err(e) => (e.to_string(), true),
    }
}

impl KafkaState {
    pub fn refresh_conns(&mut self) { self.connections = load_config().kafka; }
    pub fn current(&self) -> Option<&KafkaConn> { self.connections.get(self.selected) }

    pub fn fetch_info(&mut self) {
        if let Some(c) = self.current().cloned() {
            self.fetch_topics();
            self.fetch_groups();
            self.find_logs();
            self.output = format!("Brokers: {}\nTopics: {}个\nConsumer Groups: {}个\n日志: {}",
                c.brokers, self.topics.len(), self.groups.len(),
                if self.logs.is_empty() { "未找到".to_string() } else { format!("{}条", self.logs.len()) });
        }
    }

    pub fn fetch_topics(&mut self) {
        if let Some(c) = self.current().cloned() {
            let (out, _) = run_kafka_cmd(&c.brokers, &["--list"]);
            self.topics.clear();
            for t in out.lines() {
                let t = t.trim();
                if t.is_empty() || t.starts_with("__") { continue; }
                let (desc, _) = run_kafka_cmd(&c.brokers, &["--describe", "--topic", t]);
                let parts = desc.lines().filter(|l| l.contains("Partition:")).count() as i32;
                let repl = desc.lines().filter(|l| l.contains("Replicas:")).count() as i32;
                self.topics.push((t.to_string(), parts.max(1), repl.max(1)));
            }
        }
    }

    pub fn fetch_groups(&mut self) {
        if let Some(c) = self.current().cloned() {
            let (out, _) = run_kafka_groups(&c.brokers);
            self.groups.clear();
            for l in out.lines() {
                let l = l.trim();
                if l.is_empty() { continue; }
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() >= 2 {
                    self.groups.push((parts[0].to_string(), parts.get(1).unwrap_or(&"-").to_string()));
                } else if !l.is_empty() {
                    self.groups.push((l.to_string(), "-".into()));
                }
            }
        }
    }

    pub fn find_logs(&mut self) {
        let candidates = ["/var/log/kafka/", "/opt/kafka/logs/", "/usr/local/kafka/logs/"];
        for p in candidates {
            if std::path::Path::new(p).exists() { self.log_path = p.to_string(); break; }
        }
        if !self.log_path.is_empty() && std::path::Path::new(&self.log_path).is_dir() {
            if let Ok(entries) = std::fs::read_dir(&self.log_path) {
                for e in entries.flatten() {
                    let n = e.file_name().to_string_lossy().to_string();
                    if n.contains("server") && n.ends_with(".log") {
                        self.log_path = e.path().to_string_lossy().to_string();
                        break;
                    }
                }
            }
        }
        self.tail_logs();
    }

    pub fn tail_logs(&mut self) {
        if self.log_path.is_empty() { return; }
        if let Ok(out) = Command::new("tail").arg("-n").arg("50").arg(&self.log_path).output() {
            self.logs = String::from_utf8_lossy(&out.stdout).lines().map(|l| l.to_string()).collect();
        }
    }

    pub fn connect_new(&mut self) {
        let conn = KafkaConn {
            name: if self.config_name.is_empty() { self.config_brokers.clone() } else { self.config_name.clone() },
            brokers: self.config_brokers.clone(),
            sasl_user: if self.config_user.is_empty() { None } else { Some(self.config_user.clone()) },
            sasl_password: if self.config_pass.is_empty() { None } else { Some(self.config_pass.clone()) },
        };
        let name = conn.name.clone(); add_kafka_conn(conn);
        self.refresh_conns();
        self.selected = self.connections.iter().position(|c| c.name == name).unwrap_or(0);
        self.mode = KafkaMode::Normal; self.fetch_info();
    }
}

fn dm(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.text_dim)) }
fn dp(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.primary).bold()) }
fn dt(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.text)) }

pub fn render_kafka_overview(f: &mut Frame, area: Rect, state: &mut KafkaState, t: &Theme) {
    let [sidebar, main] = Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)]).areas(area);
    let sb = Block::default().title(dp(t, "Kafka 连接")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(sb.clone(), sidebar);
    let si = sb.inner(sidebar).inner(Margin::new(1, 0));
    let items: Vec<ListItem> = state.connections.iter().enumerate().map(|(i, c)| {
        let s = if i == state.selected { Style::default().fg(t.primary).bg(t.surface_alt).bold() } else { Style::default().fg(t.text) };
        ListItem::new(format!(" {} {}", if i == state.selected { "▶" } else { " " }, c.name)).style(s)
    }).collect();
    let mut ls = ListState::default(); ls.select(Some(state.selected));
    f.render_stateful_widget(List::new(items), si, &mut ls);

    let mb = Block::default().title(dp(t, "Kafka 信息")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(mb.clone(), main);
    let mi = mb.inner(main).inner(Margin::new(1, 1));
    if let Some(c) = state.current() {
        let mut lines = vec![Line::from(dp(t, &format!("◉ {}", c.brokers)))];
        lines.push(Line::from(dm(t, "──────────────────────────────────────────────")));
        if !state.output.is_empty() {
            for l in state.output.lines() { lines.push(Line::from(dt(t, l))); }
        } else { lines.push(Line::from(dm(t, "  (回车连接并刷新)"))); }
        lines.push(Line::from(""));
        lines.push(Line::from(dm(t, "t:Topics  g:Groups  l:日志  a:添加  Enter:连接刷新")));
        f.render_widget(Paragraph::new(lines), mi);
    } else {
        f.render_widget(Paragraph::new("  没有Kafka连接\n  a: 添加新连接").centered(), mi);
    }
}

pub fn render_kafka_topics(f: &mut Frame, area: Rect, state: &KafkaState, t: &Theme) {
    let b = Block::default().title(dp(t, &format!("Kafka Topics ({}个)", state.topics.len())))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(), area);
    let i = b.inner(area).inner(Margin::new(1, 0));
    let mut lines = vec![Line::from(vec![dm(t, " TOPIC                    "), dm(t, "PARTITIONS"), dm(t, "  REPLICAS")])];
    lines.push(Line::from(dm(t, "──────────────────────────────────────────────────")));
    for (topic, parts, repl) in state.topics.iter().skip(state.scroll).take(35) {
        lines.push(Line::from(vec![
            dt(t, &format!(" {:<24}", ts(topic, 24))),
            dt(t, &format!("  {:>4}", parts)),
            dt(t, &format!("      {:>2}", repl)),
        ]));
    }
    if state.topics.is_empty() { lines.push(Line::from(dm(t, "  (无topic或未连接)"))); }
    f.render_widget(Paragraph::new(lines), i);
}

pub fn render_kafka_groups(f: &mut Frame, area: Rect, state: &KafkaState, t: &Theme) {
    let b = Block::default().title(dp(t, &format!("Consumer Groups ({}个)", state.groups.len())))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(), area);
    let i = b.inner(area).inner(Margin::new(1, 0));
    let mut lines = vec![Line::from(vec![dm(t, " GROUP                    "), dm(t, "STATE")])];
    lines.push(Line::from(dm(t, "──────────────────────────────────────")));
    for (g, s) in state.groups.iter().skip(state.scroll).take(35) {
        let sc = if s.contains("Stable") { t.success } else { t.warning };
        lines.push(Line::from(vec![
            dt(t, &format!(" {:<24}", ts(g, 24))),
            Span::styled(s.clone(), Style::default().fg(sc)),
        ]));
    }
    if state.groups.is_empty() { lines.push(Line::from(dm(t, "  (无group或未连接)"))); }
    f.render_widget(Paragraph::new(lines), i);
}

pub fn render_kafka_logs(f: &mut Frame, area: Rect, state: &KafkaState, t: &Theme) {
    let b = Block::default().title(dp(t, &format!("Kafka 日志 ({})", ts(&state.log_path, 40))))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.warning)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(), area);
    let i = b.inner(area).inner(Margin::new(1, 0));
    let lines: Vec<Line> = state.logs.iter().skip(state.scroll).take(35).map(|l| {
        let c = if l.contains("ERROR") || l.contains("FATAL") { t.error }
        else if l.contains("WARN") { t.warning } else { t.text_dim };
        Line::from(Span::styled(ts(l, 110), Style::default().fg(c)))
    }).collect();
    if lines.is_empty() { f.render_widget(Paragraph::new("  无日志\n  r: 重新检测日志"), i); }
    else { f.render_widget(Paragraph::new(lines), i); }
}

pub fn render_kafka_add(f: &mut Frame, area: Rect, state: &mut KafkaState, t: &Theme) {
    let b = Block::default().title(dp(t, "添加 Kafka 连接")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.primary)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(), area);
    let i = b.inner(area).inner(Margin::new(2, 2));
    let fields: [(&str, &str, u8); 4] = [
        ("名称", &state.config_name, 0), ("Brokers", &state.config_brokers, 1),
        ("SASL User", &state.config_user, 2),
        ("SASL Pass", if state.config_pass.is_empty() { "(无)" } else { "***" }, 3),
    ];
    let lines: Vec<Line> = fields.iter().map(|(label, val, fi)| {
        let (pre, post) = if state.edit_field == *fi { ("▶ ", " ▍") } else { ("  ", "") };
        Line::from(vec![
            Span::styled(format!("{}{:<12}", pre, label), if state.edit_field == *fi { Style::default().fg(t.primary).bold() } else { Style::default().fg(t.text_dim) }),
            Span::styled(format!("{}{}", val, post), Style::default().fg(t.text)),
        ])
    }).collect();
    f.render_widget(Paragraph::new(lines), i);
}

fn ts(s: &str, n: usize) -> String {let cs:Vec<char>=s.chars().collect();if cs.len()<=n{s.into()}else{format!("{}…",cs.iter().take(n-1).collect::<String>())}}

