use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use super::config::{load_config, add_redis_conn, RedisConn};
use std::process::Command;

const REDIS_COMMANDS: &[&str] = &[
    "AUTH","BGREWRITEAOF","BGSAVE","CLIENT","CLUSTER","COMMAND","CONFIG","DBSIZE",
    "DEBUG","DECR","DECRBY","DEL","DISCARD","DUMP","ECHO","EVAL","EVALSHA","EXEC",
    "EXISTS","EXPIRE","EXPIREAT","FLUSHALL","FLUSHDB","GEOADD","GEODIST","GEOHASH",
    "GEOPOS","GEORADIUS","GET","GETBIT","GETRANGE","GETSET","HDEL","HEXISTS","HGET",
    "HGETALL","HINCRBY","HKEYS","HLEN","HMGET","HMSET","HSET","HSETNX","HSTRLEN",
    "HVALS","INCR","INCRBY","INFO","KEYS","LASTSAVE","LINDEX","LINSERT","LLEN",
    "LPOP","LPUSH","LRANGE","LREM","LSET","LTRIM","MEMORY","MGET","MONITOR","MOVE",
    "MSET","OBJECT","PERSIST","PEXPIRE","PFADD","PFCOUNT","PING","PSETEX","PTTL",
    "PUBLISH","QUIT","RENAME","RESTORE","ROLE","RPOP","RPUSH","SADD","SAVE","SCARD",
    "SDIFF","SELECT","SET","SETBIT","SETEX","SETNX","SETRANGE","SINTER","SISMEMBER",
    "SLOWLOG","SMEMBERS","SORT","SPOP","SRANDMEMBER","SREM","STRLEN","SUNION",
    "SWAPDB","SYNC","TIME","TTL","TYPE","UNLINK","UNWATCH","WATCH","XADD","XLEN",
    "XREAD","XREVRANGE","ZADD","ZCARD","ZCOUNT","ZINCRBY","ZRANGE","ZRANK",
    "ZREM","ZREMRANGEBYRANK","ZREVRANGE","ZSCORE","SCAN","SSCAN","HSCAN","ZSCAN",
];

#[derive(Clone)]
pub struct RedisState {
    pub connections: Vec<RedisConn>,
    pub selected: usize,
    pub info: String,
    pub output: String,
    pub output_err: bool,
    pub cli_input: String,
    pub cli_history: Vec<String>,
    pub cli_hist_idx: usize,
    pub mode: RedisMode,
    pub config_name: String,
    pub config_host: String,
    pub config_port: String,
    pub config_pass: String,
    pub config_db: String,
    pub keys: Vec<(String, String, i64)>,
    pub key_filter: String,
    pub key_detail: String,
    pub scroll: usize,
    pub edit_field: u8,
    pub confirm_delete: bool,
    pub selected_key: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub enum RedisMode { Normal, AddConn, Cli, Keyspace, KeyDetail }

impl Default for RedisState {
    fn default() -> Self {
        Self {
            connections: vec![], selected: 0, info: String::new(), output: String::new(),
            output_err: false, cli_input: String::new(), cli_history: vec![], cli_hist_idx: 0,
            mode: RedisMode::Normal, config_name: String::new(), config_host: "127.0.0.1".into(),
            config_port: "6379".into(), config_pass: String::new(), config_db: "0".into(),
            keys: vec![], key_filter: String::new(), key_detail: String::new(),
            scroll: 0, edit_field: 0, confirm_delete: false, selected_key: 0,
        }
    }
}

impl RedisState {
    pub fn refresh_conns(&mut self) {
        self.connections = load_config().redis;
    }
    pub fn current(&self) -> Option<&RedisConn> { self.connections.get(self.selected) }

    fn redis_cmd(conn: &RedisConn, args: &[&str]) -> (String, bool) {
        let mut cmd = Command::new("redis-cli");
        cmd.arg("-h").arg(&conn.host).arg("-p").arg(conn.port.to_string());
        if let Some(ref pw) = conn.password { cmd.arg("-a").arg(pw); }
        cmd.arg("-n").arg(conn.db.to_string()).arg("--no-raw");
        for a in args { cmd.arg(a); }
        match cmd.output() {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout).to_string();
                let e = String::from_utf8_lossy(&o.stderr).to_string();
                if s.is_empty() && !e.is_empty() { (e, true) } else { (s, false) }
            }
            Err(e) => (e.to_string(), true),
        }
    }

    pub fn fetch_info(&mut self) {
        if let Some(c) = self.current().cloned() {
            self.info = Self::redis_cmd(&c, &["INFO"]).0;
        }
    }

    pub fn fetch_keys(&mut self) {
        if let Some(c) = self.current().cloned() {
            let pat = if self.key_filter.is_empty() { "*" } else { &self.key_filter };
            let (out, _) = Self::redis_cmd(&c, &["KEYS", pat]);
            self.keys.clear();
            for k in out.lines().take(200) {
                let k = k.trim();
                if k.is_empty() { continue; }
                let (typ, _) = Self::redis_cmd(&c, &["TYPE", k]);
                let (ttl_s, _) = Self::redis_cmd(&c, &["TTL", k]);
                let ttl: i64 = ttl_s.trim().parse().unwrap_or(-2);
                self.keys.push((k.to_string(), typ.trim().to_string(), ttl));
            }
        }
    }

    pub fn fetch_key_detail(&mut self, key: &str) {
        if let Some(c) = self.current().cloned() {
            let (typ, _) = Self::redis_cmd(&c, &["TYPE", key]);
            let typ = typ.trim().to_string();
            let mut detail = format!("Key: {}\nType: {}\n", key, typ);
            let (ttl, _) = Self::redis_cmd(&c, &["TTL", key]);
            detail.push_str(&format!("TTL: {}\n", ttl.trim()));
            match typ.as_str() {
                "string" => { let (v,_)=Self::redis_cmd(&c,&["GET",key]); detail.push_str(&format!("Value: {}",v)); }
                "list" => { let (v,_)=Self::redis_cmd(&c,&["LLEN",key]); let (vs,_)=Self::redis_cmd(&c,&["LRANGE",key,"0","99"]); detail.push_str(&format!("Len: {}\n{}",v.trim(),vs)); }
                "set" => { let (v,_)=Self::redis_cmd(&c,&["SCARD",key]); let (vs,_)=Self::redis_cmd(&c,&["SMEMBERS",key]); detail.push_str(&format!("Size: {}\n{}",v.trim(),vs)); }
                "hash" => { let (v,_)=Self::redis_cmd(&c,&["HLEN",key]); let (vs,_)=Self::redis_cmd(&c,&["HGETALL",key]); detail.push_str(&format!("Fields: {}\n{}",v.trim(),vs)); }
                "zset" => { let (v,_)=Self::redis_cmd(&c,&["ZCARD",key]); let (vs,_)=Self::redis_cmd(&c,&["ZRANGE",key,"0","99","WITHSCORES"]); detail.push_str(&format!("Size: {}\n{}",v.trim(),vs)); }
                _ => {}
            }
            self.key_detail = detail;
        }
    }

    pub fn del_key(&mut self, key: &str) {
        if let Some(c) = self.current().cloned() {
            let _ = Self::redis_cmd(&c, &["DEL", key]);
            self.fetch_keys();
        }
    }

    pub fn exec_cli(&mut self) {
        if let Some(c) = self.current().cloned() {
            let cmd = self.cli_input.clone();
            if cmd.is_empty() { return; }
            self.cli_history.push(cmd.clone());
            self.cli_hist_idx = self.cli_history.len();
            let args: Vec<&str> = cmd.split_whitespace().collect();
            let (out, err) = Self::redis_cmd(&c, &args);
            self.output = out;
            self.output_err = err;
            self.cli_input.clear();
        }
    }

    pub fn autocomplete(&self) -> Vec<String> {
        if self.cli_input.is_empty() { return vec![]; }
        let upper = self.cli_input.to_uppercase();
        REDIS_COMMANDS.iter().filter(|c| c.starts_with(&upper)).map(|c| c.to_string()).collect()
    }

    pub fn connect_new(&mut self) {
        let conn = RedisConn {
            name: if self.config_name.is_empty() { format!("{}:{}", self.config_host, self.config_port) }
            else { self.config_name.clone() },
            host: self.config_host.clone(),
            port: self.config_port.parse().unwrap_or(6379),
            password: if self.config_pass.is_empty() { None } else { Some(self.config_pass.clone()) },
            db: self.config_db.parse().unwrap_or(0),
        };
        let (ping, _) = Self::redis_cmd(&conn, &["PING"]);
        let name = conn.name.clone();
        add_redis_conn(conn);
        self.refresh_conns();
        self.selected = self.connections.iter().position(|c| c.name == name).unwrap_or(0);
        self.output = format!("PING → {}", ping.trim());
        self.output_err = !ping.contains("PONG");
        self.mode = RedisMode::Normal;
        self.fetch_info();
    }
}

// ---- Render functions ----

fn dm(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.text_dim)) }
fn dp(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.primary).bold()) }
fn dt(t: &Theme, s: &str) -> Span<'static> { Span::styled(s.to_string(), Style::default().fg(t.text)) }

pub fn render_redis_overview(f: &mut Frame, area: Rect, state: &mut RedisState, t: &Theme) {
    let [sidebar, main] = Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)]).areas(area);
    let sb = Block::default().title(dp(t, "Redis 连接")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(sb.clone(), sidebar);
    let si = sb.inner(sidebar).inner(Margin::new(1, 0));
    let items: Vec<ListItem> = state.connections.iter().enumerate().map(|(i, c)| {
        let s = if i == state.selected { Style::default().fg(t.primary).bg(t.surface_alt).bold() }
        else { Style::default().fg(t.text) };
        ListItem::new(format!(" {} {}:{}", if i == state.selected { "▶" } else { " " }, c.host, c.port)).style(s)
    }).collect();
    let mut ls = ListState::default(); ls.select(Some(state.selected));
    f.render_stateful_widget(List::new(items), si, &mut ls);

    let mb = Block::default().title(dp(t, "Redis 信息")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(mb.clone(), main);
    let mi = mb.inner(main).inner(Margin::new(1, 1));
    if let Some(c) = state.current() {
        let mut lines = vec![Line::from(dp(t, &format!("◉ {}:{}  db={}", c.host, c.port, c.db)))];
        lines.push(Line::from(dm(t, "──────────────────────────────────────────────")));
        if !state.info.is_empty() {
            let mut ver=""; let mut mem=""; let mut clients=""; let mut ops="";
            let mut ks=""; let mut upt=""; let mut role=""; let mut aof="";
            for l in state.info.lines() {
                if l.starts_with("redis_version:") { ver = l.trim_start_matches("redis_version:"); }
                if l.starts_with("used_memory_human:") { mem = l.trim_start_matches("used_memory_human:"); }
                if l.starts_with("connected_clients:") { clients = l.trim_start_matches("connected_clients:"); }
                if l.starts_with("instantaneous_ops_per_sec:") { ops = l.trim_start_matches("instantaneous_ops_per_sec:"); }
                if l.starts_with("uptime_in_seconds:") { upt = l.trim_start_matches("uptime_in_seconds:"); }
                if l.starts_with("role:") { role = l.trim_start_matches("role:"); }
                if l.starts_with("aof_enabled:") { aof = l.trim_start_matches("aof_enabled:"); }
                if l.starts_with("db0:") { ks = l.trim_start_matches("db0:"); }
            }
            let us: u64 = upt.trim().parse().unwrap_or(0);
            lines.push(Line::from(vec![dm(t,"版本:"),dt(t,ver),dm(t,"  角色:"),dt(t,role)]));
            lines.push(Line::from(vec![dm(t,"内存:"),dt(t,mem),dm(t,"  AOF:"),dt(t,aof)]));
            lines.push(Line::from(vec![dm(t,"客户端:"),dt(t,clients),dm(t,"  操作/秒:"),dt(t,ops)]));
            lines.push(Line::from(vec![dm(t,"运行:"),dt(t,&format!("{}d{}h{}m",us/86400,(us%86400)/3600,(us%3600)/60))]));
            if !ks.is_empty() { lines.push(Line::from(vec![dm(t,"Key空间:"),dt(t,ks)])); }
        } else { lines.push(Line::from(dm(t, "  (回车刷新 INFO)"))); }
        lines.push(Line::from(""));
        lines.push(Line::from(dm(t, "i:INFO  c:CLI  k:Key空间  a:添加连接  d:删除连接")));
        f.render_widget(Paragraph::new(lines), mi);
    } else {
        f.render_widget(Paragraph::new("  没有Redis连接\n  a: 添加新连接").centered(), mi);
    }
}

pub fn render_redis_cli(f: &mut Frame, area: Rect, state: &mut RedisState, t: &Theme) {
    let [out_a, input_a] = Layout::vertical([Constraint::Percentage(85), Constraint::Percentage(15)]).areas(area);
    let ob = Block::default().title(dp(t, "Redis CLI")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(ob.clone(), out_a);
    let oi = ob.inner(out_a).inner(Margin::new(1, 1));
    let ot = if state.output.is_empty() { Text::styled("redis> 输入命令 (Tab 补全, Enter 执行, Esc 返回)", Style::default().fg(t.text_muted)) }
    else if state.output_err { Text::styled(&state.output, Style::default().fg(t.error)) }
    else { Text::styled(&state.output, Style::default().fg(t.text)) };
    f.render_widget(Paragraph::new(ot).wrap(Wrap{trim:false}), oi);

    let ib = Block::default().title("输入").borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.primary)).style(Style::default().bg(t.surface));
    f.render_widget(ib.clone(), input_a);
    let ii = ib.inner(input_a).inner(Margin::new(1, 1));
    let it = if state.cli_input.is_empty() { Text::styled("redis> _", Style::default().fg(t.text_muted)) }
    else { Text::styled(format!("redis> {}▍", state.cli_input), Style::default().fg(t.text)) };
    f.render_widget(Paragraph::new(it), ii);

    let comps = state.autocomplete();
    if !comps.is_empty() {
        let hint = comps.iter().take(6).cloned().collect::<Vec<_>>().join(" | ");
        f.render_widget(Paragraph::new(dm(t, &format!("  {}", hint))),
            Rect{y: area.y+area.height.saturating_sub(2), height:1, ..area});
    }
}

pub fn render_redis_keyspace(f: &mut Frame, area: Rect, state: &mut RedisState, t: &Theme) {
    let [filter_a, keys_a] = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).areas(area);
    let fb = Block::default().title(dp(t, "Key 过滤")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(fb.clone(), filter_a);
    let fi = fb.inner(filter_a).inner(Margin::new(1, 0));
    let ft = if state.key_filter.is_empty() { Text::styled("  * (输入过滤, Enter 刷新, Esc 返回)", Style::default().fg(t.text_muted)) }
    else { Text::styled(format!("  {}▍", state.key_filter), Style::default().fg(t.text)) };
    f.render_widget(Paragraph::new(ft), fi);

    let kb = Block::default().title(dp(t, &format!("Key 空间 ({}个)", state.keys.len()))).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(kb.clone(), keys_a);
    let ki = kb.inner(keys_a).inner(Margin::new(1, 0));
    let items: Vec<ListItem> = state.keys.iter().skip(state.scroll).take(40).enumerate().map(|(i,(k,typ,ttl))| {
        let color = match typ.as_str() { "string"=>t.success,"list"=>t.secondary,"set"=>t.warning,"hash"=>t.primary,"zset"=>t.accent,_=>t.text_dim };
        let s = if i == state.selected_key { Style::default().fg(color).bg(t.surface_alt).bold() }
        else { Style::default().fg(color) };
        let ttl_str = if *ttl == -1 { "永久".into() } else if *ttl == -2 { "过期".into() } else { format!("{}s", ttl) };
        ListItem::new(Line::from(vec![
            Span::styled(format!(" {:<6}", typ), s),
            Span::styled(trunc_s(k, 35), Style::default().fg(t.text)),
            Span::styled(format!(" TTL:{}", ttl_str), Style::default().fg(t.text_dim)),
        ]))
    }).collect();
    let mut ls = ListState::default(); ls.select(Some(state.selected_key));
    f.render_stateful_widget(List::new(items), ki, &mut ls);
}

pub fn render_redis_add(f: &mut Frame, area: Rect, state: &mut RedisState, t: &Theme) {
    let b = Block::default().title(dp(t, "添加 Redis 连接")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.primary)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(), area);
    let i = b.inner(area).inner(Margin::new(2, 2));
    let fields = [
        ("名称", state.config_name.as_str(), 0u8),
        ("Host", state.config_host.as_str(), 1u8),
        ("Port", state.config_port.as_str(), 2u8),
        ("Password", if state.config_pass.is_empty() { "(无密码)" } else { "***" }, 3u8),
        ("DB", state.config_db.as_str(), 4u8),
    ];
    let lines: Vec<Line> = fields.iter().map(|(label, val, fidx)| {
        let (pre, post) = if state.edit_field == *fidx { ("▶ ", " ▍") } else { ("  ", "") };
        Line::from(vec![
            Span::styled(format!("{}{:<12}", pre, label), if state.edit_field == *fidx { Style::default().fg(t.primary).bold() } else { Style::default().fg(t.text_dim) }),
            Span::styled(format!("{}{}", val, post), Style::default().fg(t.text)),
        ])
    }).collect();
    f.render_widget(Paragraph::new(lines), i);
}

pub fn render_redis_key_detail(f: &mut Frame, area: Rect, state: &RedisState, t: &Theme) {
    let b = Block::default().title(dp(t, "Key 详情")).borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(), area);
    let i = b.inner(area).inner(Margin::new(1, 1));
    f.render_widget(Paragraph::new(Text::styled(&state.key_detail, Style::default().fg(t.text))).wrap(Wrap{trim:false}), i);
}

fn trunc_s(s: &str, n: usize) -> String {let cs:Vec<char>=s.chars().collect();if cs.len()<=n{s.into()}else{format!("{}…",cs.iter().take(n-1).collect::<String>())}}

