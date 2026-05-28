// ES Manager: cluster info, index management, real-time log streaming with auto-discovery
// Called by middleware/mod.rs line 3, and app.rs for rendering
// No direct file I/O; delegates to config.rs for persistence

use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use super::config::{load_config, add_es_conn, EsConn};
use std::process::Command;

#[derive(Clone)]
pub struct EsState {
    pub connections: Vec<EsConn>, pub selected: usize, pub mode: EsMode,
    pub info: String, pub indices: Vec<(String,String,String,String)>,
    pub output: String, pub scroll: usize, pub logs: Vec<String>, pub log_path: String,
    pub config_name: String, pub config_host: String, pub config_port: String,
    pub config_user: String, pub config_pass: String, pub config_scheme: String,
    pub edit_field: u8,
}

#[derive(Clone, Copy, PartialEq)]
pub enum EsMode { Normal, AddConn, Indices, Logs }

impl Default for EsState {
    fn default() -> Self {
        Self {
            connections: vec![], selected: 0, mode: EsMode::Normal, info: String::new(),
            indices: vec![], output: String::new(), scroll: 0, logs: vec![],
            log_path: String::new(), config_name: String::new(), config_host: "127.0.0.1".into(),
            config_port: "9200".into(), config_user: String::new(), config_pass: String::new(),
            config_scheme: "http".into(), edit_field: 0,
        }
    }
}

fn es_curl(url: &str, user: Option<&str>, pass: Option<&str>) -> (String, bool) {
    let mut cmd = Command::new("curl");
    cmd.arg("-s").arg("-X").arg("GET");
    if let (Some(u), Some(p)) = (user, pass) {
        if !u.is_empty() { cmd.arg("-u").arg(format!("{}:{}", u, p)); }
    }
    cmd.arg(url).arg("--connect-timeout").arg("5").arg("--max-time").arg("10");
    match cmd.output() {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            (s, !o.status.success())
        }
        Err(e) => (e.to_string(), true),
    }
}

impl EsState {
    pub fn refresh_conns(&mut self) { self.connections = load_config().elasticsearch; }
    pub fn current(&self) -> Option<&EsConn> { self.connections.get(self.selected) }

    fn base_url(conn: &EsConn) -> String { format!("{}://{}:{}", conn.scheme, conn.host, conn.port) }

    pub fn fetch_info(&mut self) {
        if let Some(c) = self.current().cloned() {
            let url = format!("{}/", Self::base_url(&c));
            let (info, _) = es_curl(&url, c.user.as_deref(), c.password.as_deref());
            self.info = info;
            let url2 = format!("{}/_cluster/health", Self::base_url(&c));
            let (health, _) = es_curl(&url2, c.user.as_deref(), c.password.as_deref());
            self.output = health;
        }
    }

    pub fn fetch_indices(&mut self) {
        if let Some(c) = self.current().cloned() {
            let url = format!("{}/_cat/indices?format=json&h=index,docs.count,store.size,health", Self::base_url(&c));
            let (out, _) = es_curl(&url, c.user.as_deref(), c.password.as_deref());
            self.indices.clear();
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&out) {
                for v in arr {
                    let name = v.get("index").and_then(|v|v.as_str()).unwrap_or("-").to_string();
                    let docs = v.get("docs.count").and_then(|v|v.as_str()).unwrap_or("-").to_string();
                    let size = v.get("store.size").and_then(|v|v.as_str()).unwrap_or("-").to_string();
                    let health = v.get("health").and_then(|v|v.as_str()).unwrap_or("-").to_string();
                    self.indices.push((name, docs, size, health));
                }
            }
        }
    }

    pub fn find_logs(&mut self) {
        let candidates = ["/var/log/elasticsearch/","/usr/share/elasticsearch/logs/","/opt/elasticsearch/logs/"];
        self.log_path.clear();
        for p in candidates {
            if std::path::Path::new(p).exists() {
                self.log_path = p.to_string(); break;
            }
        }
        if !self.log_path.is_empty() && std::path::Path::new(&self.log_path).is_dir() {
            if let Ok(entries) = std::fs::read_dir(&self.log_path) {
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.ends_with(".log") && !name.contains("slow") && !name.contains("gc") {
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
            let s = String::from_utf8_lossy(&out.stdout).to_string();
            self.logs = s.lines().map(|l|l.to_string()).collect();
        }
    }

    pub fn connect_new(&mut self) {
        let conn = EsConn {
            name: if self.config_name.is_empty() { format!("{}:{}",self.config_host,self.config_port) }
            else { self.config_name.clone() },
            host: self.config_host.clone(), port: self.config_port.parse().unwrap_or(9200),
            user: if self.config_user.is_empty(){None}else{Some(self.config_user.clone())},
            password: if self.config_pass.is_empty(){None}else{Some(self.config_pass.clone())},
            scheme: if self.config_scheme.is_empty(){"http".into()}else{self.config_scheme.clone()},
        };
        let name = conn.name.clone(); add_es_conn(conn);
        self.refresh_conns();
        self.selected = self.connections.iter().position(|c|c.name==name).unwrap_or(0);
        self.mode = EsMode::Normal; self.fetch_info(); self.find_logs();
    }
}

fn dm(t:&Theme,s:&str)->Span<'static> {Span::styled(s.to_string(),Style::default().fg(t.text_dim))}
fn dp(t:&Theme,s:&str)->Span<'static> {Span::styled(s.to_string(),Style::default().fg(t.primary).bold())}
fn dt(t:&Theme,s:&str)->Span<'static> {Span::styled(s.to_string(),Style::default().fg(t.text))}

pub fn render_es_overview(f:&mut Frame,area:Rect,state:&mut EsState,t:&Theme){
    let[sidebar,main]=Layout::horizontal([Constraint::Percentage(25),Constraint::Percentage(75)]).areas(area);
    let sb=Block::default().title(dp(t,"ES 连接")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(sb.clone(),sidebar);
    let si=sb.inner(sidebar).inner(Margin::new(1,0));
    let items:Vec<ListItem>=state.connections.iter().enumerate().map(|(i,c)|{
        let s=if i==state.selected{Style::default().fg(t.primary).bg(t.surface_alt).bold()}else{Style::default().fg(t.text)};
        ListItem::new(format!(" {} {}:{}",if i==state.selected{"▶"}else{" "},c.host,c.port)).style(s)
    }).collect();
    let mut ls=ListState::default();ls.select(Some(state.selected));
    f.render_stateful_widget(List::new(items),si,&mut ls);

    let mb=Block::default().title(dp(t,"Elasticsearch")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(mb.clone(),main);
    let mi=mb.inner(main).inner(Margin::new(1,1));
    if let Some(c)=state.current(){
        let mut lines=vec![Line::from(dp(t,&format!("◉ {}://{}:{}",c.scheme,c.host,c.port)))];
        lines.push(Line::from(dm(t,"──────────────────────────────────────────────")));
        if !state.info.is_empty(){
            let iv:serde_json::Value=serde_json::from_str(&state.info).unwrap_or_default();
            let name=iv.get("cluster_name").and_then(|v|v.as_str()).unwrap_or("-");
            let ver=iv.get("version").and_then(|v|v.get("number")).and_then(|v|v.as_str()).unwrap_or("-");
            lines.push(Line::from(vec![dm(t,"集群:"),dt(t,name),dm(t,"  版本:"),dt(t,ver)]));
            if !state.output.is_empty(){
                let h:serde_json::Value=serde_json::from_str(&state.output).unwrap_or_default();
                let st=h.get("status").and_then(|v|v.as_str()).unwrap_or("-").to_string();
                let nodes=h.get("number_of_nodes").and_then(|v|v.as_str()).unwrap_or("-").to_string();
                let dn=h.get("number_of_data_nodes").and_then(|v|v.as_str()).unwrap_or("-").to_string();
                let sh=h.get("active_shards").and_then(|v|v.as_str()).unwrap_or("-").to_string();
                let sc=match st.as_str(){"green"=>t.success,"yellow"=>t.warning,_=>t.error};
                lines.push(Line::from(vec![dm(t,"状态:"),Span::styled(st,Style::default().fg(sc).bold()),dm(t,"  节点:"),dt(t,&nodes)]));
                lines.push(Line::from(vec![dm(t,"数据节点:"),dt(t,&dn),dm(t,"  分片:"),dt(t,&sh)]));
            }
            let log_status=if !state.logs.is_empty(){format!("{}条",state.logs.len())}else{"未找到".into()};
            lines.push(Line::from(vec![dm(t,"日志:"),dt(t,&log_status),dm(t,"  "),dt(t,if state.log_path.is_empty(){"(检测中)"}else{&state.log_path})]));
        }else{lines.push(Line::from(dm(t,"  (回车连接并刷新)")));}
        lines.push(Line::from(""));
        lines.push(Line::from(dm(t,"i:INFO  n:索引  l:日志  r:重检日志  a:添加  Esc:返回")));
        f.render_widget(Paragraph::new(lines),mi);
    }else{f.render_widget(Paragraph::new("  没有ES连接\n  a: 添加新连接").centered(),mi);}
}

pub fn render_es_indices(f:&mut Frame,area:Rect,state:&EsState,t:&Theme){
    let b=Block::default().title(dp(t,&format!("ES 索引 ({}个)",state.indices.len()))).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(),area);
    let i=b.inner(area).inner(Margin::new(1,0));
    let mut lines=vec![Line::from(vec![dm(t," INDEX                            "),dm(t,"DOCS      "),dm(t,"SIZE      "),dm(t,"HEALTH")])];
    lines.push(Line::from(dm(t,"────────────────────────────────────────────────────────────────")));
    for(name,docs,size,health)in state.indices.iter().skip(state.scroll).take(35){
        let hc=match health.as_str(){"green"=>t.success,"yellow"=>t.warning,_=>t.error};
        lines.push(Line::from(vec![dt(t,&format!(" {:<32}",ts(name,32))),dt(t,&format!(" {:<10}",docs)),dt(t,&format!(" {:<10}",size)),Span::styled(health.clone(),Style::default().fg(hc).bold())]));
    }
    if state.indices.is_empty(){lines.push(Line::from(dm(t,"  (无索引或未连接)")));}
    f.render_widget(Paragraph::new(lines),i);
}

pub fn render_es_logs(f:&mut Frame,area:Rect,state:&EsState,t:&Theme){
    let b=Block::default().title(dp(t,&format!("ES 实时日志 ({})",ts(&state.log_path,50)))).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.warning)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(),area);
    let i=b.inner(area).inner(Margin::new(1,0));
    let lines:Vec<Line>=state.logs.iter().skip(state.scroll).take(35).map(|l|{
        let c=if l.contains("ERROR")||l.contains("FATAL"){t.error}else if l.contains("WARN"){t.warning}else if l.contains("INFO"){t.text}else{t.text_dim};
        Line::from(Span::styled(ts(l,110),Style::default().fg(c)))
    }).collect();
    if lines.is_empty(){f.render_widget(Paragraph::new("  无日志\n  r: 重新检测日志"),i);}
    else{f.render_widget(Paragraph::new(lines),i);}
}

pub fn render_es_add(f:&mut Frame,area:Rect,state:&mut EsState,t:&Theme){
    let b=Block::default().title(dp(t,"添加 ES 连接")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.primary)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(),area);
    let i=b.inner(area).inner(Margin::new(2,2));
    let fields:[(&str,&str,u8);6]=[("名称",&state.config_name,0),("Host",&state.config_host,1),("Port",&state.config_port,2),("User",&state.config_user,3),("Password",if state.config_pass.is_empty(){"(无)"}else{"***"},4),("Scheme",&state.config_scheme,5)];
    let lines:Vec<Line>=fields.iter().map(|(label,val,fi)|{
        let(pre,post)=if state.edit_field==*fi{("▶ "," ▍")}else{("  ","")};
        Line::from(vec![Span::styled(format!("{}{:<10}",pre,label),if state.edit_field==*fi{Style::default().fg(t.primary).bold()}else{Style::default().fg(t.text_dim)}),Span::styled(format!("{}{}",val,post),Style::default().fg(t.text))])
    }).collect();
    f.render_widget(Paragraph::new(lines),i);
}

fn ts(s:&str,n:usize)->String{let cs:Vec<char>=s.chars().collect();if cs.len()<=n{s.into()}else{format!("{}…",cs.iter().take(n-1).collect::<String>())}}
