use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};
use devtool_core::manager::PluginManager;
use devtool_core::types::{PluginInput, PluginMetadata};
use std::collections::HashMap;
use crate::dashboard::{AppCategory,Dashboard,ErrorSeverity};
use crate::middleware::MiddlewareKind;
use crate::middleware::discovery::{self,DiscoveredService,ServiceStatus};
use crate::middleware::redis::{RedisState,RedisMode,render_redis_overview,render_redis_cli,render_redis_keyspace,render_redis_add,render_redis_key_detail};
use crate::middleware::es::{EsState,EsMode,render_es_overview,render_es_indices,render_es_logs,render_es_add};
use crate::middleware::kafka::{KafkaState,KafkaMode,render_kafka_overview,render_kafka_topics,render_kafka_groups,render_kafka_logs,render_kafka_add};
use crate::middleware::nginx::{NginxState,NginxMode,render_nginx_overview,render_nginx_config,render_nginx_logs,render_nginx_add};
use crate::middleware::tomcat::{TomcatState,TomcatMode,render_tomcat_overview,render_tomcat_config,render_tomcat_logs,render_tomcat_add};
use crate::middleware::caddy::{CaddyState,CaddyMode,render_caddy_overview,render_caddy_config,render_caddy_logs,render_caddy_add};
use crate::middleware::docker::{DockerState,DockerMode,render_docker_overview,render_docker_inspect,render_docker_logs};
use crate::theme::{Theme, DARK};

#[derive(Clone, Copy, PartialEq)] enum View { Dashboard, Plugins, Docker, Middleware }
#[derive(Clone, Copy, PartialEq)] enum Focus { Sidebar, Actions }
enum Mode { Normal, Palette, Help }

pub struct App {
    manager: PluginManager, plugins: Vec<PluginMetadata>,
    plugin_idx: usize, action_idx: usize, focus: Focus, view: View,
    dashboard: Dashboard, sidebar_filter: String,
    palette_state: ListState, palette_query: String, palette_matches: Vec<(usize, usize)>,
    mode: Mode, input_buf: String, output_buf: String, output_err: bool,
    app_scroll: usize, status: String, quit: bool,
    // Middleware + Docker states
    mw_kind: MiddlewareKind, mw_sel: usize, redis: RedisState, es: EsState, kafka: KafkaState,
    nginx: NginxState, tomcat: TomcatState, caddy: CaddyState, docker: DockerState,
    discovered: Vec<DiscoveredService>, disc_loaded: bool, mw_subpanel: usize,
}

impl App {
    pub fn new(manager: PluginManager) -> Self {
        let _ = manager.load_all();
        let plugins: Vec<_> = manager.list_plugins().into_iter().map(|(m,_)| m).collect();
        let mut redis = RedisState::default(); redis.refresh_conns();
        let mut es = EsState::default(); es.refresh_conns();
        let mut kafka = KafkaState::default(); kafka.refresh_conns();
        let mut nginx = NginxState::default(); nginx.refresh_conns();
        let mut tomcat = TomcatState::default(); tomcat.refresh_conns();
        let mut caddy = CaddyState::default(); caddy.refresh_conns();
        let docker = DockerState::default();
        Self{manager,plugins,plugin_idx:0,action_idx:0,focus:Focus::Actions,view:View::Dashboard,
            dashboard:Dashboard::new(),sidebar_filter:String::new(),
            palette_state:ListState::default(),palette_query:String::new(),palette_matches:Vec::new(),
            mode:Mode::Normal,input_buf:String::new(),output_buf:String::new(),output_err:false,
            app_scroll:0,status:"Tab切换视图 ↑↓选择 Enter运行 /搜索 q退出".into(),quit:false,
            mw_kind:MiddlewareKind::Overview,mw_sel:0,redis,es,kafka,nginx,tomcat,caddy,docker,
            discovered:Vec::new(),disc_loaded:false,mw_subpanel:0}
    }

    pub fn run(mut self, term: &mut ratatui::Terminal<impl ratatui::backend::Backend>) -> Result<(), Box<dyn std::error::Error>> {
        while !self.quit { self.dashboard.tick(); term.draw(|f| self.render(f))?; self.handle_event()?; }
        Ok(())
    }

    fn render(&mut self, f: &mut Frame) {
        let t = &DARK;
        f.render_widget(Block::default().style(Style::default().bg(t.bg)), f.area());
        match self.view { View::Dashboard=>self.render_dashboard(f,t), View::Plugins=>self.render_plugins(f,t), View::Docker=>self.render_docker_view(f,t), View::Middleware=>self.render_middleware_view(f,t) }
        let [_,footer]=Layout::vertical([Constraint::Min(1),Constraint::Length(1)]).areas(f.area());
        self.render_footer(f,footer,t);
        if matches!(self.mode,Mode::Palette){self.render_palette(f,t);}
        if matches!(self.mode,Mode::Help){self.render_help(f,t);}
    }

    // ═══════ DASHBOARD (production, no gauges) ═══════
        fn render_dashboard(&mut self, f: &mut Frame, t: &Theme) {
        let a=f.area().inner(Margin::new(1,0));let d=&self.dashboard.data;
        let dm=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.text_dim))};
        let dt=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.text))};
        let dp=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.primary).bold())};
        let da=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.accent).bold())};
        let bar=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.border))};
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::styled(" \u{25c6} ",Style::default().fg(t.primary).bold()),
            Span::styled(&d.os,Style::default().fg(t.primary).bold()),
            bar(" \u{2502} "),
            Span::styled(&d.hostname,Style::default().fg(t.accent)),
            bar(" \u{2502} "),
            Span::styled(&d.kernel,Style::default().fg(t.text_dim)),
            bar(" \u{2502} "),
            Span::styled(&d.arch,Style::default().fg(t.secondary)),
            bar(" \u{2502} "),
            Span::styled(format!("{} {}",d.hw_vendor,d.hw_serial),Style::default().fg(t.text_muted)),
            bar("  \u{2502}  "),
            Span::styled("\u{25b6}插件",Style::default().fg(t.text_dim)),
            bar(" \u{2502} "),
            Span::styled("\u{25b6}Docker",Style::default().fg(t.text_dim)),
            bar(" \u{2502} "),
            Span::styled("\u{25b6}中间件",Style::default().fg(t.text_dim)),
        ])),Rect{height:1,..a});
        let bd=Rect{y:a.y+1,height:a.height.saturating_sub(1),..a};
        let[top,bot]=Layout::vertical([Constraint::Percentage(55),Constraint::Percentage(45)]).areas(bd);
        let[left,right]=Layout::horizontal([Constraint::Percentage(50),Constraint::Percentage(50)]).areas(top);
        let[sc,na]=Layout::vertical([Constraint::Percentage(58),Constraint::Percentage(42)]).areas(left);
        let[ma,da2]=Layout::vertical([Constraint::Percentage(58),Constraint::Percentage(42)]).areas(right);
        // System/CPU (shorter + top3 CPU procs)
        let cb=Block::default().title(Line::from(vec![Span::styled(" \u{25b2} ",Style::default().fg(t.success).bold()),dp("系统 \u{2502} CPU")])).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
        f.render_widget(cb.clone(),sc);let ci=cb.inner(sc).inner(Margin::new(1,1));
        let cc=if d.cpu_pct>80.0{t.error}else if d.cpu_pct>50.0{t.warning}else{t.success};
        let lds=format!("{:.2} {:.2} {:.2}",d.load1,d.load5,d.load15);
        let cpuinf=format!("{}核@{:.0}MHz {}",d.cpu_cores,d.cpu_mhz,d.arch);let cpus=format!("{:.1}%",d.cpu_pct);
        let cpumm=format!(" 最高{:.1}% 最低{:.1}%",d.cpu_max,d.cpu_min);
        let mut cpu_lines=vec![
            Line::from(vec![dm("OS:  "),dt(&d.os),dm("  Load:"),dm(&lds)]),
            Line::from(vec![dm("CPU: "),dt(&cpuinf),dm("  Use:"),Span::styled(&cpus,Style::default().fg(cc).bold()),dm(&cpumm)]),
            Line::from(dp("▸ TOP3 CPU")),
            Line::from(dm("  ─────────────────────────────────────────")),
            Line::from(vec![dm("  PID    "),dm("NAME                         "),dm("CPU%   ")]),
        ];
        for (i,p) in d.top_cpu.iter().take(3).enumerate(){let rank=vec![t.error,t.warning,t.primary][i];let ps=format!("{:.1}%",p.cpu);cpu_lines.push(Line::from(vec![dm(&format!("  {:<7}",p.pid)),dt(&format!(" {:<27}",trunc(&p.name,27))),Span::styled(format!("{:<7}",ps),Style::default().fg(rank).bold())]));}
        f.render_widget(Paragraph::new(cpu_lines),ci);
        // Network with interface details and IPs
        let nb=Block::default().title(Line::from(vec![Span::styled(" \u{25cf} ",Style::default().fg(t.accent).bold()),dp("网络")])).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
        f.render_widget(nb.clone(),na);let ni=nb.inner(na).inner(Margin::new(1,1));
        let mut nl=vec![
            Line::from(vec![
                Span::styled(format!("↓ {}/s",fmt_rate(d.net_rx_rate)),Style::default().fg(t.accent).bold()),
                Span::styled(format!("  ↑ {}/s",fmt_rate(d.net_tx_rate)),Style::default().fg(t.warning).bold()),
            ]),
            Line::from(dm("────────────────────────────────────────────")),
            Line::from(vec![dm("IFACE     "),dm("RX(总计)  "),dm("TX(总计)  "),dm("IP")]),
            Line::from(dm("────────────────────────────────────────────")),
        ];
        for(n2,rx2,tx2)in d.ifaces.iter().take(4){
            let ip_if = d.iface_ips.iter().find(|(iface,_)| iface==n2).map(|(_,ip)| ip.as_str()).unwrap_or("-");
            nl.push(Line::from(vec![dm(&format!("{:<10}",n2)),dm(&format!("{:<10}",fmt_bytes(*rx2 as f64))),dm(&format!("{:<10}",fmt_bytes(*tx2 as f64))),dt(&format!("  {}",ip_if))]));
        }
        f.render_widget(Paragraph::new(nl),ni);
        // Memory (shorter + top3 mem procs)
        let mp=if d.mem_total>0{d.mem_used as f64/d.mem_total as f64*100.0}else{0.0};let mc=if mp>90.0{t.error}else if mp>70.0{t.warning}else{t.success};
        let mb=Block::default().title(Line::from(vec![Span::styled(" \u{25a3} ",Style::default().fg(t.warning).bold()),dp("内存")])).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
        f.render_widget(mb.clone(),ma);let mi=mb.inner(ma).inner(Margin::new(1,1));
        let mu=fmt_mem(d.mem_used);let mt=fmt_mem(d.mem_total);let ma2=fmt_mem(d.mem_avail);let mc2=fmt_mem(d.buf_cache);
        let ms=fmt_mem(d.swap_used);let mst=fmt_mem(d.swap_total);
        let ps=format!("{}({:.1}%) {}线程 FD{}/{} Z{}",d.procs,if d.procs_max>0{d.procs as f64/d.procs_max as f64*100.0}else{0.0},d.threads,d.fd_cur,fmt_big(d.fd_max),d.zombies);
        let u1=format!(" / {} ({:.1}%)",mt,mp);let u2=format!("  Cache:{}  Swap:{}/{}",mc2,ms,mst);
        let mut mem_lines=vec![
            Line::from(vec![dm("Used: "),Span::styled(mu.as_str(),Style::default().fg(mc).bold()),dm(&u1)]),
            Line::from(vec![dm("Avail:"),dt(ma2.as_str()),dm(&u2),dm("  "),dm(&ps)]),
            Line::from(dp("▸ TOP3 内存")),
            Line::from(dm("  ─────────────────────────────────────────")),
            Line::from(vec![dm("  PID    "),dm("NAME                         "),dm("MEM    ")]),
        ];
        for (i,p) in d.top_mem.iter().take(3).enumerate(){let rank=vec![t.error,t.warning,t.secondary][i];let ms=fmt_kb(p.mem_kb);mem_lines.push(Line::from(vec![dm(&format!("  {:<7}",p.pid)),dt(&format!(" {:<27}",trunc(&p.name,27))),Span::styled(format!("{:<7}",ms),Style::default().fg(rank).bold())]));}
        f.render_widget(Paragraph::new(mem_lines),mi);
        // Disk - FULL df -h format with ALL partitions
        let db=Block::default().title(Line::from(vec![Span::styled(" \u{25c7} ",Style::default().fg(t.secondary).bold()),dp("磁盘")])).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
        f.render_widget(db.clone(),da2);let di=db.inner(da2).inner(Margin::new(1,0));
        let hdr=Span::styled(format!(" {:<18} {:<7} {:<7} {:<7} {:<5} {}","Filesystem","Size","Used","Avail","Use%","Mounted on"),Style::default().fg(t.primary).bold());
        let mut dl=vec![Line::from(hdr)];
        for(dv,sz,us,pct)in d.disks.iter(){let pct_n:f64=pct.trim_end_matches('%').parse().unwrap_or(0.0);let dc=if pct_n>90.0{t.error}else if pct_n>70.0{t.warning}else{t.success};let ds=format!(" {:<18} {:<7} {:<7} {:<7} {:<5} /{}",dv,sz,us,if pct=="100%"{"100%"}else{pct},pct,dv.split('/').last().unwrap_or(dv));dl.push(Line::from(Span::styled(ds,Style::default().fg(dc))));}
        if d.disks.is_empty(){dl.push(Line::from(dm("  (no physical disks found)")));}
        f.render_widget(Paragraph::new(dl),di);
        // Bottom: App Status FIRST, then Error Log + Anomaly
        let[apps_a,err_anom]=Layout::vertical([Constraint::Percentage(50),Constraint::Percentage(50)]).areas(bot);
        // App Status TABLE with borders
        let ab2=Block::default().title(Line::from(vec![Span::styled(" \u{25b8} ",Style::default().fg(t.accent).bold()),da("应用状态"),Span::styled(" (非docker)",Style::default().fg(t.text_muted))])).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
        f.render_widget(ab2.clone(),apps_a);
        let widths=[Constraint::Length(6),Constraint::Length(8),Constraint::Length(6),Constraint::Length(7),Constraint::Length(4),Constraint::Length(8),Constraint::Length(5),Constraint::Min(35)];
        let header=Row::new(vec![Cell::from("PID"),Cell::from("PORTS"),Cell::from("CPU%"),Cell::from("MEM"),Cell::from("THR"),Cell::from("USER"),Cell::from("TYPE"),Cell::from("COMMAND")]).style(Style::default().fg(t.primary).bold());
        let rows:Vec<Row>=d.apps.iter().skip(self.app_scroll).map(|a2|{
            let sc2=match a2.category{AppCategory::Java=>Style::default().fg(t.accent).bold(),AppCategory::WebServer=>Style::default().fg(t.secondary).bold(),AppCategory::Database=>Style::default().fg(t.warning).bold(),AppCategory::Cache=>Style::default().fg(t.success).bold(),AppCategory::Search=>Style::default().fg(t.primary).bold(),AppCategory::MessageQueue=>Style::default().fg(t.warning),AppCategory::Container=>Style::default().fg(t.text_dim),AppCategory::Other=>Style::default().fg(t.text_muted)};
            let lb=match a2.category{AppCategory::Java=>"Java",AppCategory::WebServer=>"Web",AppCategory::Database=>"DB",AppCategory::Cache=>"Cache",AppCategory::MessageQueue=>"MQ",AppCategory::Search=>"Search",AppCategory::Container=>"CTR",AppCategory::Other=>"?"};
            let ports_str = if a2.ports.is_empty(){"-".into()}else{a2.ports.iter().map(|p|p.to_string()).collect::<Vec<_>>().join(",")};
            Row::new(vec![Cell::from(format!("{}",a2.pid)),Cell::from(ports_str),Cell::from(format!("{:.1}%",a2.cpu_pct)),Cell::from(fmt_kb(a2.mem_kb)),Cell::from(format!("{}",a2.threads)),Cell::from(a2.user.clone()),Cell::from(lb),Cell::from(trunc(&a2.name,80))]).style(sc2)
        }).collect();
        f.render_stateful_widget(Table::new(rows,widths).header(header),ab2.inner(apps_a).inner(Margin::new(0,0)),&mut TableState::default());
        // Error Log + Anomaly
        let[errs,anom]=Layout::horizontal([Constraint::Percentage(70),Constraint::Percentage(30)]).areas(err_anom);
        let eb=Block::default().title(Line::from(vec![Span::styled(" \u{25c8} ",Style::default().fg(t.error).bold()),Span::styled(format!("系统错误日志 ({})",d.sys_errors.len()),Style::default().fg(t.error).bold())])).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.error)).style(Style::default().bg(t.surface));
        f.render_widget(eb.clone(),errs);
        let mut el:Vec<ListItem>=d.sys_errors.iter().map(|e|{
            let(icon,styl)=match e.severity{ErrorSeverity::Critical=>("🔴",Style::default().fg(t.error).bold()),ErrorSeverity::Error=>("🟡",Style::default().fg(t.warning)),ErrorSeverity::Warning=>("⚪",Style::default().fg(t.text_dim))};
            ListItem::new(format!(" {} {:<15} {}",icon,e.service,trunc(&e.message,70))).style(styl)
        }).collect();
        if el.is_empty(){el.push(ListItem::new("  ✓ 未检测到系统错误").style(Style::default().fg(t.success)));}
        f.render_stateful_widget(List::new(el),eb.inner(errs).inner(Margin::new(0,0)),&mut ListState::default());
        let at2=if d.anomalies.is_empty(){" 系统实时状态监控 ✓ ".to_string()}else{format!(" 系统实时状态监控 ⚠({}) ",d.anomalies.len())};
        let ac2=if d.anomalies.is_empty(){t.success}else{t.error};
        let ab3=Block::default().title(Span::styled(at2,Style::default().fg(ac2).bold())).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(if d.anomalies.is_empty(){t.border}else{t.error})).style(Style::default().bg(t.surface));
        f.render_widget(ab3.clone(),anom);
        let ai3:Vec<ListItem>=if d.anomalies.is_empty(){vec![ListItem::new("  ✓ 未检测到异常").style(Style::default().fg(t.success))]}else{d.anomalies.iter().map(|s|ListItem::new(format!("  {}",s)).style(Style::default().fg(t.error))).collect()};
        f.render_stateful_widget(List::new(ai3),ab3.inner(anom).inner(Margin::new(0,0)),&mut ListState::default());
        if !self.output_buf.is_empty(){let oa=centered_rect(70,60,f.area());f.render_widget(Clear,oa);let ob=Block::default().title(Span::styled(" 输出 ",Style::default().fg(t.success).bold())).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.success)).style(Style::default().bg(t.surface_alt));f.render_widget(Paragraph::new(if self.output_err{Span::styled(&self.output_buf,Style::default().fg(t.error))}else{Span::styled(&self.output_buf,Style::default().fg(t.text))}).wrap(Wrap{trim:false}).block(ob),oa);}
    }

    fn render_plugins(&mut self, f: &mut Frame, t: &Theme) {
        let area = f.area();
        f.render_widget(Paragraph::new(Line::from(vec![Span::styled(" ← 仪表盘  ",Style::default().fg(t.primary).bold()),Span::styled(format!("{}个插件",self.plugins.len()),Style::default().fg(t.text_dim))])),Rect{height:1,..area});
        let w=Rect{y:area.y+1,height:area.height.saturating_sub(1),..area};
        let has_in=self.needs_input();
        let (body_a,input_a)=if has_in{let[b,i]=Layout::vertical([Constraint::Percentage(80),Constraint::Percentage(20)]).areas(w);(b,Some(i))}else{(w,None)};
        let[sidebar,main]=Layout::horizontal([Constraint::Percentage(22),Constraint::Percentage(78)]).areas(body_a);
        let[detail,out_a]=Layout::vertical([Constraint::Percentage(40),Constraint::Percentage(60)]).areas(main);
        self.render_sidebar(f,sidebar,t); self.render_detail(f,detail,t);
        if let Some(ia)=input_a{let ib=Block::default().title("输入").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));f.render_widget(ib.clone(),ia);let txt=if self.input_buf.is_empty(){Text::styled("打字输入",Style::default().fg(t.text_muted))}else{Text::styled(format!("{}▍",self.input_buf),Style::default().fg(t.text))};f.render_widget(Paragraph::new(txt),ib.inner(ia).inner(Margin::new(1,1)));}
        let ob=Block::default().title("输出").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));f.render_widget(ob.clone(),out_a);
        let ot=if self.output_buf.is_empty(){Text::styled("选择功能→Enter",Style::default().fg(t.text_muted))}else if self.output_err{Text::styled(&self.output_buf,Style::default().fg(t.error))}else{Text::styled(&self.output_buf,Style::default().fg(t.text))};
        f.render_widget(Paragraph::new(ot).wrap(Wrap{trim:false}),ob.inner(out_a).inner(Margin::new(1,1)));
    }
    fn render_sidebar(&mut self, f: &mut Frame, area: Rect, t: &Theme) {
        let fq=self.sidebar_filter.to_lowercase(); let vis:Vec<(usize,&PluginMetadata)>=self.plugins.iter().enumerate().filter(|(_,p)|fq.is_empty()||p.name.to_lowercase().contains(&fq)).collect();
        let sidebar_focused=self.focus==Focus::Sidebar;
        let b=Block::default().title(Line::from(vec![
            Span::styled(if sidebar_focused{"\u{25c0} "}else{"\u{25b6} "},Style::default().fg(if sidebar_focused{t.primary}else{t.accent}).bold()),
            Span::styled(if sidebar_focused{format!("插件[{}]",self.sidebar_filter)}else{"插件".into()},Style::default().fg(if sidebar_focused{t.primary}else{t.accent}).bold()),
            Span::styled(format!(" ({})",vis.len()),Style::default().fg(t.text_muted)),
        ])).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(if sidebar_focused{t.primary}else{t.border})).style(Style::default().bg(t.surface));
        f.render_widget(b.clone(),area); let inner=b.inner(area);
        if vis.is_empty(){f.render_widget(Paragraph::new("无匹配").centered(),inner);return;}
        let items:Vec<ListItem>=vis.iter().map(|(i,p)|{
            let sel=*i==self.plugin_idx;
            let cat_color=match &p.category{devtool_core::types::PluginCategory::DataTool=>t.secondary,devtool_core::types::PluginCategory::SystemTool=>t.success,devtool_core::types::PluginCategory::Security=>t.error,devtool_core::types::PluginCategory::Middleware=>t.warning,devtool_core::types::PluginCategory::Script=>t.accent,devtool_core::types::PluginCategory::Network=>t.primary,_=>t.text_dim};
            let sty=if sel{Style::default().fg(t.primary).bg(t.surface_alt).bold()}else{Style::default().fg(t.text)};
            ListItem::new(Line::from(vec![
                Span::styled(if sel{"\u{25b6} "}else{"  "},Style::default().fg(if sel{t.primary}else{t.surface})),
                Span::styled(format!("{} ",cat_tag(&p.category)),Style::default().fg(cat_color).bold()),
                Span::styled(&p.name,sty),
            ]))
        }).collect();
        let mut st=ListState::default(); if let Some(p)=vis.iter().position(|(i,_)|*i==self.plugin_idx){st.select(Some(p));}
        f.render_stateful_widget(List::new(items),inner,&mut st);
    }
    fn render_detail(&self, f: &mut Frame, area: Rect, t: &Theme) {
        let b=Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(if self.focus==Focus::Actions{t.primary}else{t.border})).style(Style::default().bg(t.surface));
        f.render_widget(b.clone(),area); let inner=b.inner(area).inner(Margin::new(1,1));
        if let Some(p)=self.plugins.get(self.plugin_idx){let mut lines=vec![Line::from(vec![Span::styled(&p.name,Style::default().fg(t.primary).bold()),Span::styled(format!(" v{}",p.version),Style::default().fg(t.text_muted))]),Line::from(Span::styled(&p.description,Style::default().fg(t.text_dim))),Line::from(""),Line::from(Span::styled(if self.focus==Focus::Actions{"◀功能"}else{"功能"},Style::default().fg(t.accent).bold()))];for(i,a)in p.actions.iter().enumerate(){let sel=i==self.action_idx;let s=if sel{Style::default().fg(t.primary).bg(t.surface_alt).bold()}else{Style::default().fg(t.text)};lines.push(Line::from(vec![Span::styled(if sel{"▶"}else{" "},s),Span::styled(format!(" {}",a.name),s),Span::styled(format!("  {}",a.description),Style::default().fg(t.text_dim))]));}f.render_widget(Paragraph::new(lines).wrap(Wrap{trim:false}),inner);}
    }
    fn needs_input(&self)->bool{if !self.input_buf.is_empty(){return true;}self.plugins.get(self.plugin_idx).and_then(|p|p.actions.get(self.action_idx)).map(|a|!a.params.is_empty()).unwrap_or(false)}

    fn render_footer(&self,f:&mut Frame,area:Rect,t:&Theme){let hint=match self.mode{Mode::Palette=>"Esc:关闭 Enter:执行",Mode::Help=>"任意键关闭",Mode::Normal=>match self.view{View::Dashboard=>"→:插件 Tab:Docker/MW /:搜索 r:重载 q:退出",View::Plugins=>match self.focus{Focus::Sidebar=>"←:仪表盘 →:功能 ↑↓:选择 打字:过滤",Focus::Actions=>"←:仪表盘 ↑↓:选择 打字:输入 Enter:执行"},View::Docker=>"↑↓:容器 s:启动 p:停止 t:重启 x:删除 i:详情 l:日志 r:刷新 Esc:返回",View::Middleware=>"←→:中间件 ↑↓:选择 Enter:进入 1-9:快速跳转 Esc:返回"}};f.render_widget(Paragraph::new(Line::from(vec![Span::styled(format!(" {} ",hint),Style::default().fg(t.text_dim)),Span::styled("\u{2502}",Style::default().fg(t.border)),Span::styled(format!(" {} ",self.status),Style::default().fg(t.accent))])).style(Style::default().bg(t.surface)),area);}

    fn handle_event(&mut self) -> Result<(),Box<dyn std::error::Error>>{if !crossterm::event::poll(std::time::Duration::from_millis(200))?{return Ok(());}match crossterm::event::read()?{Event::Key(k)=>{if k.kind==KeyEventKind::Release{return Ok(());}if k.code==KeyCode::Char('c')&&k.modifiers.contains(KeyModifiers::CONTROL){self.quit=true;return Ok(());}match self.mode{Mode::Palette=>self.handle_palette(k),Mode::Help=>{self.mode=Mode::Normal;},Mode::Normal=>self.handle_normal(k)}}Event::Mouse(m)=>{if matches!(m.kind,MouseEventKind::ScrollDown){self.app_scroll=self.app_scroll.saturating_add(1)}else if matches!(m.kind,MouseEventKind::ScrollUp){self.app_scroll=self.app_scroll.saturating_sub(1)}else if matches!(m.kind,MouseEventKind::Down(crossterm::event::MouseButton::Right)){self.cycle_view_forward();}}_=>{}}Ok(())}
    fn handle_normal(&mut self,k:crossterm::event::KeyEvent){
        // Check if a view-specific handler wants this key first
        if self.view==View::Docker&&self.handle_docker_key(k){return;}
        if self.view==View::Middleware&&self.mw_kind!=MiddlewareKind::Overview&&self.handle_mw_detail_key(k){return;}
        // Common navigation
        match k.code{
        KeyCode::Char('q')=>self.quit=true,
        KeyCode::Esc=>{if !self.input_buf.is_empty(){self.input_buf.clear();}else if self.view!=View::Dashboard{self.view=View::Dashboard;}else{self.quit=true;}}
        KeyCode::Char('?')=>self.mode=Mode::Help,
        KeyCode::Char('/')=>{self.open_palette();}
        KeyCode::Char('r')=>{self.reload();}
        KeyCode::Tab|KeyCode::Char('\t')=>{self.cycle_view_forward();}
        KeyCode::BackTab=>{self.cycle_view_backward();}
        KeyCode::Right if self.view==View::Dashboard=>{self.view=View::Plugins;self.focus=Focus::Actions;}
        KeyCode::Right if self.view==View::Plugins=>{self.sidebar_filter.clear();self.view=View::Docker;}
        KeyCode::Right if self.view==View::Docker=>{self.view=View::Middleware;self.mw_kind=MiddlewareKind::Overview;}
        KeyCode::Right if self.view==View::Middleware=>{self.mw_kind=MiddlewareKind::Overview;self.view=View::Dashboard;}
        KeyCode::Left if self.view==View::Plugins=>{self.sidebar_filter.clear();self.view=View::Dashboard;}
        KeyCode::Left if self.view==View::Docker=>{self.view=View::Plugins;self.sidebar_filter.clear();}
        KeyCode::Left if self.view==View::Middleware=>{self.mw_kind=MiddlewareKind::Overview;self.view=View::Docker;}
        KeyCode::Left if self.view==View::Dashboard=>{self.view=View::Middleware;self.mw_kind=MiddlewareKind::Overview;}
        KeyCode::Enter=>{
            match self.view{
                View::Dashboard=>{self.view=View::Plugins;self.focus=Focus::Actions;}
                View::Plugins=>{self.execute();}
                View::Middleware if self.mw_kind==MiddlewareKind::Overview=>{let kinds=MiddlewareKind::manageable();self.mw_kind=kinds[self.mw_sel.min(kinds.len()-1)];self.mw_enter_detail();}
                _=>{}
            }
        }
        KeyCode::Up=>{
            if self.view==View::Middleware&&self.mw_kind==MiddlewareKind::Overview{let max=MiddlewareKind::manageable().len()-1;self.mw_sel=if self.mw_sel==0{max}else{self.mw_sel-1};}
            else{self.move_up();}
        }
        KeyCode::Down=>{
            if self.view==View::Middleware&&self.mw_kind==MiddlewareKind::Overview{let max=MiddlewareKind::manageable().len()-1;self.mw_sel=if self.mw_sel>=max{0}else{self.mw_sel+1};}
            else{self.move_down();}
        }
        KeyCode::Char('k')=>{self.move_up();}
        KeyCode::Char('j')=>{self.move_down();}
        KeyCode::Backspace|KeyCode::Delete=>match(self.view,self.focus){(View::Plugins,Focus::Sidebar)=>{self.sidebar_filter.pop();}_=>{self.input_buf.pop();}}
        KeyCode::Char('h') if k.modifiers.contains(KeyModifiers::CONTROL)=>{self.input_buf.pop();}
        KeyCode::Char('w') if k.modifiers.contains(KeyModifiers::CONTROL)=>{let mut w:Vec<&str>=self.input_buf.split_whitespace().collect();w.pop();self.input_buf=w.join(" ");}
        KeyCode::Char('u') if k.modifiers.contains(KeyModifiers::CONTROL)=>{self.input_buf.clear();}
        KeyCode::Char('1') if self.view==View::Middleware=>{self.mw_jump(0);}
        KeyCode::Char('2') if self.view==View::Middleware=>{self.mw_jump(1);}
        KeyCode::Char('3') if self.view==View::Middleware=>{self.mw_jump(2);}
        KeyCode::Char('4') if self.view==View::Middleware=>{self.mw_jump(3);}
        KeyCode::Char('5') if self.view==View::Middleware=>{self.mw_jump(4);}
        KeyCode::Char('6') if self.view==View::Middleware=>{self.mw_jump(5);}
        KeyCode::Char('7') if self.view==View::Middleware=>{self.mw_jump(6);}
        KeyCode::Char('8') if self.view==View::Middleware=>{self.mw_jump(7);}
        KeyCode::Char('9') if self.view==View::Middleware=>{self.mw_jump(8);}
        KeyCode::Char(c)=>match(self.view,self.focus){(View::Plugins,Focus::Sidebar)=>{self.sidebar_filter.push(c);}_=>{self.input_buf.push(c);}}
        _=>{}}}
    fn cycle_view_forward(&mut self){self.view=match self.view{View::Dashboard=>{self.focus=Focus::Actions;View::Plugins},View::Plugins=>{self.sidebar_filter.clear();View::Docker},View::Docker=>{self.mw_kind=MiddlewareKind::Overview;View::Middleware},View::Middleware=>{self.mw_kind=MiddlewareKind::Overview;View::Dashboard}}}
    fn cycle_view_backward(&mut self){self.view=match self.view{View::Dashboard=>{self.mw_kind=MiddlewareKind::Overview;View::Middleware},View::Middleware=>View::Docker,View::Docker=>View::Plugins,View::Plugins=>{self.sidebar_filter.clear();View::Dashboard}}}
    fn move_up(&mut self){if self.view!=View::Plugins{return;}match self.focus{Focus::Sidebar=>{let v=self.filtered();if let Some(p)=v.iter().position(|&i|i==self.plugin_idx){self.plugin_idx=v[if p==0{v.len().saturating_sub(1)}else{p-1}];}else if !v.is_empty(){self.plugin_idx=v[0];}self.action_idx=0;}Focus::Actions=>{if let Some(p)=self.plugins.get(self.plugin_idx){if p.actions.is_empty(){return;}if self.action_idx==0{self.action_idx=p.actions.len()-1}else{self.action_idx-=1;}}}}}
    fn move_down(&mut self){if self.view!=View::Plugins{return;}match self.focus{Focus::Sidebar=>{let v=self.filtered();if let Some(p)=v.iter().position(|&i|i==self.plugin_idx){self.plugin_idx=v[if p+1>=v.len(){0}else{p+1}];}else if !v.is_empty(){self.plugin_idx=v[0];}self.action_idx=0;}Focus::Actions=>{if let Some(p)=self.plugins.get(self.plugin_idx){if p.actions.is_empty(){return;}if self.action_idx+1>=p.actions.len(){self.action_idx=0}else{self.action_idx+=1;}}}}}
    fn handle_docker_key(&mut self,k:crossterm::event::KeyEvent)->bool{match k.code{KeyCode::Char('r')=>{self.docker.fetch_all();true}KeyCode::Char('s')=>{self.docker.start();true}KeyCode::Char('p')=>{self.docker.stop();true}KeyCode::Char('t')=>{self.docker.restart();true}KeyCode::Char('x')=>{self.docker.rm();true}KeyCode::Char('i')=>{self.docker.inspect();true}KeyCode::Char('l')=>{self.docker.fetch_logs();true}KeyCode::Up=>{self.docker.selected=self.docker.selected.saturating_sub(1);true}KeyCode::Down=>{self.docker.selected=(self.docker.selected+1).min(self.docker.containers.len().saturating_sub(1));true}KeyCode::Enter=>{self.docker.inspect();true}_=>false}}
    fn handle_mw_detail_key(&mut self,k:crossterm::event::KeyEvent)->bool{match k.code{KeyCode::Esc=>{if self.is_mw_submode(){self.reset_mw_mode();}else{self.mw_kind=MiddlewareKind::Overview;}true}KeyCode::Char('r')=>{self.discovered=discovery::discover_all();self.disc_loaded=true;true}KeyCode::Char('a')=>{self.mw_enter_add();true}KeyCode::Char('i')=>{self.mw_fetch_info();true}KeyCode::Char('c')=>{self.mw_enter_cli();true}KeyCode::Char('l')=>{self.mw_enter_logs();true}KeyCode::Char('d')=>{self.mw_del_conn();true}KeyCode::Char('k') if self.mw_kind==MiddlewareKind::Redis=>{self.redis.mode=RedisMode::Keyspace;self.redis.fetch_keys();true}KeyCode::Char('t') if self.mw_kind==MiddlewareKind::Kafka=>{self.kafka.mode=KafkaMode::Topics;self.kafka.fetch_topics();true}KeyCode::Char('g') if self.mw_kind==MiddlewareKind::Kafka=>{self.kafka.mode=KafkaMode::Groups;self.kafka.fetch_groups();true}KeyCode::Char('n') if self.mw_kind==MiddlewareKind::Elasticsearch=>{self.es.mode=EsMode::Indices;self.es.fetch_indices();true}KeyCode::Char('v') if self.mw_kind==MiddlewareKind::Caddy=>{self.caddy.validate_config();true}KeyCode::Up=>{self.mw_sel_up();true}KeyCode::Down=>{self.mw_sel_down();true}KeyCode::Enter=>{self.mw_handle_enter();true}KeyCode::Backspace|KeyCode::Delete=>{self.mw_input_pop();true}KeyCode::Char(c)=>{self.mw_input_push(c);true}_=>false}}
    fn is_mw_submode(&self)->bool{match self.mw_kind{MiddlewareKind::Redis=>self.redis.mode!=RedisMode::Normal,MiddlewareKind::Elasticsearch=>self.es.mode!=EsMode::Normal,MiddlewareKind::Kafka=>self.kafka.mode!=KafkaMode::Normal,MiddlewareKind::Nginx=>self.nginx.mode!=NginxMode::Normal,MiddlewareKind::Tomcat=>self.tomcat.mode!=TomcatMode::Normal,MiddlewareKind::Caddy=>self.caddy.mode!=CaddyMode::Normal,_=>false}}
    fn reset_mw_mode(&mut self){match self.mw_kind{MiddlewareKind::Redis=>self.redis.mode=RedisMode::Normal,MiddlewareKind::Elasticsearch=>self.es.mode=EsMode::Normal,MiddlewareKind::Kafka=>self.kafka.mode=KafkaMode::Normal,MiddlewareKind::Nginx=>self.nginx.mode=NginxMode::Normal,MiddlewareKind::Tomcat=>self.tomcat.mode=TomcatMode::Normal,MiddlewareKind::Caddy=>self.caddy.mode=CaddyMode::Normal,_=>{}}}
    fn mw_enter_detail(&mut self){match self.mw_kind{MiddlewareKind::Redis=>self.redis.fetch_info(),MiddlewareKind::Elasticsearch=>{self.es.fetch_info();self.es.find_logs();}MiddlewareKind::Kafka=>self.kafka.fetch_info(),MiddlewareKind::Nginx=>self.nginx.fetch_info(),MiddlewareKind::Tomcat=>self.tomcat.fetch_info(),MiddlewareKind::Caddy=>self.caddy.fetch_info(),_=>self.ensure_discovery()}}
    fn mw_jump(&mut self,i:usize){let kinds=MiddlewareKind::manageable();if let Some(k)=kinds.get(i){self.mw_kind=*k;self.mw_sel=i;self.mw_enter_detail();}}
    fn mw_fetch_info(&mut self){match self.mw_kind{MiddlewareKind::Redis=>self.redis.fetch_info(),MiddlewareKind::Elasticsearch=>self.es.fetch_info(),MiddlewareKind::Kafka=>self.kafka.fetch_info(),MiddlewareKind::Nginx=>self.nginx.fetch_info(),MiddlewareKind::Tomcat=>self.tomcat.fetch_info(),MiddlewareKind::Caddy=>self.caddy.fetch_info(),_=>{}}}
    fn mw_enter_add(&mut self){match self.mw_kind{MiddlewareKind::Redis=>self.redis.mode=RedisMode::AddConn,MiddlewareKind::Elasticsearch=>self.es.mode=EsMode::AddConn,MiddlewareKind::Kafka=>self.kafka.mode=KafkaMode::AddConn,MiddlewareKind::Nginx=>self.nginx.mode=NginxMode::AddConn,MiddlewareKind::Tomcat=>self.tomcat.mode=TomcatMode::AddConn,MiddlewareKind::Caddy=>self.caddy.mode=CaddyMode::AddConn,_=>{}}}
    fn mw_enter_cli(&mut self){match self.mw_kind{MiddlewareKind::Redis=>{self.redis.mode=RedisMode::Cli;self.redis.output.clear();}MiddlewareKind::Elasticsearch=>{self.es.mode=EsMode::Indices;self.es.fetch_indices();}MiddlewareKind::Nginx=>{self.nginx.mode=NginxMode::ViewConfig;self.nginx.load_config();}MiddlewareKind::Tomcat=>{self.tomcat.mode=TomcatMode::ViewConfig;self.tomcat.load_config();}MiddlewareKind::Caddy=>{self.caddy.mode=CaddyMode::ViewConfig;self.caddy.load_config();}_=>{}}}
    fn mw_enter_logs(&mut self){match self.mw_kind{MiddlewareKind::Elasticsearch=>{self.es.mode=EsMode::Logs;self.es.find_logs();}MiddlewareKind::Kafka=>{self.kafka.mode=KafkaMode::Logs;self.kafka.find_logs();}MiddlewareKind::Nginx=>{self.nginx.mode=NginxMode::Logs;self.nginx.find_logs();}MiddlewareKind::Tomcat=>{self.tomcat.mode=TomcatMode::Logs;self.tomcat.find_logs();}MiddlewareKind::Caddy=>{self.caddy.mode=CaddyMode::Logs;self.caddy.find_logs();}_=>{}}}
    fn mw_del_conn(&mut self){match self.mw_kind{MiddlewareKind::Redis=>{use crate::middleware::config::remove_redis_conn;if let Some(c)=self.redis.current(){let n=c.name.clone();remove_redis_conn(&n);self.redis.refresh_conns();self.redis.selected=0;}}MiddlewareKind::Elasticsearch=>{use crate::middleware::config::remove_es_conn;if let Some(c)=self.es.current(){let n=c.name.clone();remove_es_conn(&n);self.es.refresh_conns();self.es.selected=0;}}_=>{}}}
    fn mw_sel_up(&mut self){match self.mw_kind{MiddlewareKind::Redis=>self.redis.selected=self.redis.selected.saturating_sub(1),MiddlewareKind::Elasticsearch=>self.es.selected=self.es.selected.saturating_sub(1),MiddlewareKind::Kafka=>self.kafka.selected=self.kafka.selected.saturating_sub(1),MiddlewareKind::Nginx=>self.nginx.selected=self.nginx.selected.saturating_sub(1),MiddlewareKind::Tomcat=>self.tomcat.selected=self.tomcat.selected.saturating_sub(1),MiddlewareKind::Caddy=>self.caddy.selected=self.caddy.selected.saturating_sub(1),_=>{}}}
    fn mw_sel_down(&mut self){match self.mw_kind{MiddlewareKind::Redis=>self.redis.selected=(self.redis.selected+1).min(self.redis.connections.len().saturating_sub(1)),MiddlewareKind::Elasticsearch=>self.es.selected=(self.es.selected+1).min(self.es.connections.len().saturating_sub(1)),MiddlewareKind::Kafka=>self.kafka.selected=(self.kafka.selected+1).min(self.kafka.connections.len().saturating_sub(1)),MiddlewareKind::Nginx=>self.nginx.selected=(self.nginx.selected+1).min(self.nginx.connections.len().saturating_sub(1)),MiddlewareKind::Tomcat=>self.tomcat.selected=(self.tomcat.selected+1).min(self.tomcat.connections.len().saturating_sub(1)),MiddlewareKind::Caddy=>self.caddy.selected=(self.caddy.selected+1).min(self.caddy.connections.len().saturating_sub(1)),_=>{}}}
    fn mw_handle_enter(&mut self){match self.mw_kind{MiddlewareKind::Redis if self.redis.mode==RedisMode::AddConn=>{self.redis.connect_new();}MiddlewareKind::Elasticsearch if self.es.mode==EsMode::AddConn=>{self.es.connect_new();}MiddlewareKind::Kafka if self.kafka.mode==KafkaMode::AddConn=>{self.kafka.connect_new();}MiddlewareKind::Nginx if self.nginx.mode==NginxMode::AddConn=>{self.nginx.connect_new();}MiddlewareKind::Tomcat if self.tomcat.mode==TomcatMode::AddConn=>{self.tomcat.connect_new();}MiddlewareKind::Caddy if self.caddy.mode==CaddyMode::AddConn=>{self.caddy.connect_new();}MiddlewareKind::Redis if self.redis.mode==RedisMode::Cli=>{self.redis.exec_cli();}MiddlewareKind::Redis if self.redis.mode==RedisMode::Keyspace=>{self.redis.fetch_keys();}_=>{}}}
    fn mw_input_push(&mut self,c:char){match self.mw_kind{MiddlewareKind::Redis=>match self.redis.mode{RedisMode::AddConn=>match self.redis.edit_field{0=>self.redis.config_name.push(c),1=>self.redis.config_host.push(c),2=>self.redis.config_port.push(c),3=>self.redis.config_pass.push(c),4=>self.redis.config_db.push(c),_=>{}}RedisMode::Cli=>self.redis.cli_input.push(c),RedisMode::Keyspace=>self.redis.key_filter.push(c),_=>{}},MiddlewareKind::Elasticsearch if self.es.mode==EsMode::AddConn=>match self.es.edit_field{0=>self.es.config_name.push(c),1=>self.es.config_host.push(c),2=>self.es.config_port.push(c),3=>self.es.config_user.push(c),4=>self.es.config_pass.push(c),5=>self.es.config_scheme.push(c),_=>{}},MiddlewareKind::Kafka if self.kafka.mode==KafkaMode::AddConn=>match self.kafka.edit_field{0=>self.kafka.config_name.push(c),1=>self.kafka.config_brokers.push(c),2=>self.kafka.config_user.push(c),3=>self.kafka.config_pass.push(c),_=>{}},MiddlewareKind::Nginx if self.nginx.mode==NginxMode::AddConn=>match self.nginx.edit_field{0=>self.nginx.config_name.push(c),1=>self.nginx.config_path.push(c),2=>self.nginx.log_conf.push(c),_=>{}},MiddlewareKind::Tomcat if self.tomcat.mode==TomcatMode::AddConn=>match self.tomcat.edit_field{0=>self.tomcat.config_name.push(c),1=>self.tomcat.catalina_home.push(c),2=>self.tomcat.log_conf.push(c),_=>{}},MiddlewareKind::Caddy if self.caddy.mode==CaddyMode::AddConn=>match self.caddy.edit_field{0=>self.caddy.config_name.push(c),1=>self.caddy.config_path.push(c),2=>self.caddy.log_conf.push(c),_=>{}},_=>{}}}
    fn mw_input_pop(&mut self){match self.mw_kind{MiddlewareKind::Redis=>match self.redis.mode{RedisMode::AddConn=>match self.redis.edit_field{0=>{self.redis.config_name.pop();}1=>{self.redis.config_host.pop();}2=>{self.redis.config_port.pop();}3=>{self.redis.config_pass.pop();}4=>{self.redis.config_db.pop();}_=>{}}RedisMode::Cli=>{self.redis.cli_input.pop();}RedisMode::Keyspace=>{self.redis.key_filter.pop();}_=>{}},MiddlewareKind::Elasticsearch if self.es.mode==EsMode::AddConn=>match self.es.edit_field{0=>{self.es.config_name.pop();}1=>{self.es.config_host.pop();}2=>{self.es.config_port.pop();}3=>{self.es.config_user.pop();}4=>{self.es.config_pass.pop();}5=>{self.es.config_scheme.pop();}_=>{}},MiddlewareKind::Kafka if self.kafka.mode==KafkaMode::AddConn=>match self.kafka.edit_field{0=>{self.kafka.config_name.pop();}1=>{self.kafka.config_brokers.pop();}2=>{self.kafka.config_user.pop();}3=>{self.kafka.config_pass.pop();}_=>{}},MiddlewareKind::Nginx if self.nginx.mode==NginxMode::AddConn=>match self.nginx.edit_field{0=>{self.nginx.config_name.pop();}1=>{self.nginx.config_path.pop();}2=>{self.nginx.log_conf.pop();}_=>{}},MiddlewareKind::Tomcat if self.tomcat.mode==TomcatMode::AddConn=>match self.tomcat.edit_field{0=>{self.tomcat.config_name.pop();}1=>{self.tomcat.catalina_home.pop();}2=>{self.tomcat.log_conf.pop();}_=>{}},MiddlewareKind::Caddy if self.caddy.mode==CaddyMode::AddConn=>match self.caddy.edit_field{0=>{self.caddy.config_name.pop();}1=>{self.caddy.config_path.pop();}2=>{self.caddy.log_conf.pop();}_=>{}},_=>{}}}
    fn filtered(&self)->Vec<usize>{let f=self.sidebar_filter.to_lowercase();self.plugins.iter().enumerate().filter(|(_,p)|f.is_empty()||p.name.to_lowercase().contains(&f)).map(|(i,_)|i).collect()}
    fn execute(&mut self){let Some(p)=self.plugins.get(self.plugin_idx)else{return};let Some(a)=p.actions.get(self.action_idx)else{return};let inp=if self.input_buf.is_empty(){None}else{Some(self.input_buf.clone())};match self.manager.execute(&p.name,PluginInput{action:a.name.clone(),params:HashMap::new(),input_data:inp,input_file:None}){Ok(o)=>{self.output_buf=o.data;self.output_err=!o.success;self.status=if o.success{"✓完成".into()}else{format!("✗ {}",o.error.as_deref().unwrap_or("失败"))};}Err(e)=>{self.output_buf=e.to_string();self.output_err=true;self.status=format!("✗ {}",e);}}}
    fn reload(&mut self){match self.manager.load_all(){Ok(_)=>{self.plugins=self.manager.list_plugins().into_iter().map(|(m,_)|m).collect();self.plugin_idx=self.plugin_idx.min(self.plugins.len().saturating_sub(1));self.status=format!("✓{}个插件",self.plugins.len());}Err(e)=>{self.status=format!("✗ {}",e);}}}

    fn ensure_discovery(&mut self){if !self.disc_loaded{self.discovered=discovery::discover_all();self.disc_loaded=true;}}

    fn render_docker_view(&mut self,f:&mut Frame,t:&Theme){
        let a=f.area().inner(Margin::new(1,0));
        let dm=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.text_dim))};
        let dp=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.primary).bold())};
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::styled(" 🐳 Docker ",Style::default().fg(t.primary).bold()),
            Span::styled(format!("| {}容器 {}镜像 | Tab切换视图",self.docker.containers.len(),self.docker.images.len()),Style::default().fg(t.text_muted)),
        ])),Rect{height:1,..a});
        let body=Rect{y:a.y+1,height:a.height.saturating_sub(1),..a};
        if !self.docker.loaded{
            f.render_widget(Paragraph::new("  ⏳ Loading Docker containers...\n  (首次加载，数据将被缓存)")
                .style(Style::default().fg(t.text_dim)).centered(),body);
            self.docker.fetch_all();
            return;
        }
        match self.docker.mode{
            DockerMode::Inspect=>render_docker_inspect(f,body,&self.docker,t),
            DockerMode::Logs=>render_docker_logs(f,body,&self.docker,t),
            _=>render_docker_overview(f,body,&mut self.docker,t),
        }
    }

    fn render_middleware_view(&mut self,f:&mut Frame,t:&Theme){
        self.ensure_discovery();
        let a=f.area().inner(Margin::new(1,0));
        let dm=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.text_dim))};
        let dp=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.primary).bold())};
        let dt=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.text))};
        let da=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.accent).bold())};
        let manageable=MiddlewareKind::manageable();
        let hdr=if self.mw_kind==MiddlewareKind::Overview{
            Line::from(vec![dp(" 中间件总览 "),dm(&format!("| {}种 ",manageable.len())),dm(if self.disc_loaded{"已扫描"}else{"扫描中..."}),dm(" | ↑↓选择 Enter进入 1-9跳转 Tab切换")])
        }else{
            let c=self.mw_kind.color(t);
            Line::from(vec![Span::styled(format!(" {} {} ",self.mw_kind.icon(),self.mw_kind.name()),Style::default().fg(c).bold()),dm(" | Esc:总览 Tab:切换")])
        };
        f.render_widget(Paragraph::new(hdr),Rect{height:1,..a});
        let body=Rect{y:a.y+1,height:a.height.saturating_sub(2),..a};

        if self.mw_kind==MiddlewareKind::Overview{
            let[sidebar,main]=Layout::horizontal([Constraint::Percentage(30),Constraint::Percentage(70)]).areas(body);
            // Sidebar
            let sb=Block::default().title(Line::from(vec![Span::styled(" \u{25c6} ",Style::default().fg(t.primary).bold()),dp("中间件"),Span::styled(format!(" ({})",manageable.len()),Style::default().fg(t.text_muted))])).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.primary)).style(Style::default().bg(t.surface));
            f.render_widget(sb.clone(),sidebar);
            let si=sb.inner(sidebar).inner(Margin::new(1,0));
            let items:Vec<ListItem>=manageable.iter().enumerate().map(|(i,k)|{
                let conns=self.mw_conn_count(k);
                let c=k.color(t);
                let sel=i==self.mw_sel;
                let sty=if sel{Style::default().fg(c).bg(t.surface_alt).bold()}else{Style::default().fg(t.text)};
                let num=Span::styled(format!(" {} ",i+1),Style::default().fg(if sel{c}else{t.text_muted}).bg(if sel{t.surface_alt}else{t.surface}).bold());
                let icon=Span::styled(format!("{} ",k.icon()),Style::default().fg(c));
                let name=Span::styled(format!("{:<14}",k.name()),sty);
                let count=Span::styled(format!("({})",conns),Style::default().fg(if conns>0{t.success}else{t.text_muted}));
                ListItem::new(Line::from(vec![
                    Span::styled(if sel{"\u{25b6} "}else{"  "},Style::default().fg(if sel{c}else{t.surface})),
                    icon,name,count,
                ]))
            }).collect();
            let mut ls=ListState::default();ls.select(Some(self.mw_sel));
            f.render_stateful_widget(List::new(items),si,&mut ls);
            // Main detail
            let mb=Block::default().title(dp(&format!(" {} {} ",manageable[self.mw_sel.min(manageable.len()-1)].icon(),manageable[self.mw_sel.min(manageable.len()-1)].name()))).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
            f.render_widget(mb.clone(),main);
            let mi=mb.inner(main).inner(Margin::new(1,1));
            let mk=manageable[self.mw_sel.min(manageable.len()-1)];
            let mut lines=vec![
                Line::from(vec![Span::styled(format!(" {} {} ",mk.icon(),mk.name()),Style::default().fg(mk.color(t)).bold()),dm(&format!("  ({}种连接)",self.mw_conn_count(&mk)))]),
                Line::from(Span::styled(format!(" {}", "\u{2500}".repeat(32)),Style::default().fg(t.border))),
            ];
            let related:Vec<&DiscoveredService>=self.discovered.iter().filter(|d|MiddlewareKind::from_discovery(d)==mk).collect();
            if !self.disc_loaded{lines.push(Line::from(vec![Span::styled("  \u{23f3} ",t.text_dim),dm("正在扫描...")]));}
            else if related.is_empty(){lines.push(Line::from(vec![Span::styled("  \u{26a0} ",t.warning),dm("未检测到实例"),dm("  r:重新扫描")]));}
            else{for d in related.iter().take(8){let port=d.port.map(|p|format!(":{}",p)).unwrap_or_default();let pid=d.pid.map(|p|format!(" pid:{}",p)).unwrap_or_default();let cfg=d.config_path.as_deref().unwrap_or("-");let icon=match d.status{ServiceStatus::Running=>"\u{25cf}",_=>"\u{25cb}"};let ic=match d.status{ServiceStatus::Running=>t.success,_=>t.error};lines.push(Line::from(vec![Span::styled(icon,Style::default().fg(ic).bold()),dm(" "),dt(&format!("{}{}",d.name,port)),dt(&pid),dm(" "),Span::styled(trunc(cfg,30),Style::default().fg(t.text_muted))]));}}
            lines.push(Line::from(""));
            lines.push(Line::from(vec![dm("Enter:进入  1-9:跳转  a:添加  r:扫描  Esc:总览")]));
            f.render_widget(Paragraph::new(lines),mi);
        }else{
            match self.mw_kind{
                MiddlewareKind::Redis=>self.render_redis_screen(f,body,t),
                MiddlewareKind::Elasticsearch=>self.render_es_screen(f,body,t),
                MiddlewareKind::Kafka=>self.render_kafka_screen(f,body,t),
                MiddlewareKind::Nginx=>self.render_nginx_screen(f,body,t),
                MiddlewareKind::Tomcat=>self.render_tomcat_screen(f,body,t),
                MiddlewareKind::Caddy=>self.render_caddy_screen(f,body,t),
                _=>self.render_generic_mw(f,body,t),
            }
        }
        let hint=if self.mw_kind==MiddlewareKind::Overview{"↑↓:选择  Enter:进入  1-9:跳转  a:添加  r:扫描  Esc:仪表盘  Tab:切换视图"}
        else{"Esc:总览  i:INFO  c:CLI/配置  l:日志  a:添加  d:删除  ↑↓:选连接  Tab:切换视图"};
        f.render_widget(Paragraph::new(Line::from(dm(hint))),Rect{y:body.y+body.height,height:1,..a});
    }

    fn mw_conn_count(&self,k:&MiddlewareKind)->usize{
        match k{MiddlewareKind::Redis=>self.redis.connections.len(),MiddlewareKind::Elasticsearch=>self.es.connections.len(),MiddlewareKind::Kafka=>self.kafka.connections.len(),MiddlewareKind::Nginx=>self.nginx.connections.len(),MiddlewareKind::Tomcat=>self.tomcat.connections.len(),MiddlewareKind::Caddy=>self.caddy.connections.len(),MiddlewareKind::Docker=>self.docker.containers.len(),_=>0}
    }

    fn render_redis_screen(&mut self,f:&mut Frame,area:Rect,t:&Theme){match self.redis.mode{RedisMode::AddConn=>render_redis_add(f,area,&mut self.redis,t),RedisMode::Cli=>render_redis_cli(f,area,&mut self.redis,t),RedisMode::Keyspace=>render_redis_keyspace(f,area,&mut self.redis,t),RedisMode::KeyDetail=>render_redis_key_detail(f,area,&self.redis,t),_=>render_redis_overview(f,area,&mut self.redis,t)}}
    fn render_es_screen(&mut self,f:&mut Frame,area:Rect,t:&Theme){match self.es.mode{EsMode::AddConn=>render_es_add(f,area,&mut self.es,t),EsMode::Indices=>render_es_indices(f,area,&self.es,t),EsMode::Logs=>render_es_logs(f,area,&self.es,t),_=>render_es_overview(f,area,&mut self.es,t)}}
    fn render_kafka_screen(&mut self,f:&mut Frame,area:Rect,t:&Theme){match self.kafka.mode{KafkaMode::AddConn=>render_kafka_add(f,area,&mut self.kafka,t),KafkaMode::Topics=>render_kafka_topics(f,area,&self.kafka,t),KafkaMode::Groups=>render_kafka_groups(f,area,&self.kafka,t),KafkaMode::Logs=>render_kafka_logs(f,area,&self.kafka,t),_=>render_kafka_overview(f,area,&mut self.kafka,t)}}
    fn render_nginx_screen(&mut self,f:&mut Frame,area:Rect,t:&Theme){match self.nginx.mode{NginxMode::AddConn=>render_nginx_add(f,area,&mut self.nginx,t),NginxMode::ViewConfig=>render_nginx_config(f,area,&self.nginx,t),NginxMode::Logs=>render_nginx_logs(f,area,&self.nginx,t),_=>render_nginx_overview(f,area,&mut self.nginx,t)}}
    fn render_tomcat_screen(&mut self,f:&mut Frame,area:Rect,t:&Theme){match self.tomcat.mode{TomcatMode::AddConn=>render_tomcat_add(f,area,&mut self.tomcat,t),TomcatMode::ViewConfig=>render_tomcat_config(f,area,&self.tomcat,t),TomcatMode::Logs=>render_tomcat_logs(f,area,&self.tomcat,t),_=>render_tomcat_overview(f,area,&mut self.tomcat,t)}}
    fn render_caddy_screen(&mut self,f:&mut Frame,area:Rect,t:&Theme){match self.caddy.mode{CaddyMode::AddConn=>render_caddy_add(f,area,&mut self.caddy,t),CaddyMode::ViewConfig=>render_caddy_config(f,area,&self.caddy,t),CaddyMode::Logs=>render_caddy_logs(f,area,&self.caddy,t),_=>render_caddy_overview(f,area,&mut self.caddy,t)}}

    fn render_generic_mw(&mut self,f:&mut Frame,area:Rect,t:&Theme){
        let dt=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.text))};
        let dm=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.text_dim))};
        let dp=|s:&str|->Span{Span::styled(s.to_string(),Style::default().fg(t.primary).bold())};
        let name=self.mw_kind.name();
        let related:Vec<&DiscoveredService>=self.discovered.iter().filter(|d|MiddlewareKind::from_discovery(d)==self.mw_kind).collect();
        let b=Block::default().title(dp(&format!(" {} 实例信息 ",name))).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.primary)).style(Style::default().bg(t.surface));
        f.render_widget(b.clone(),area);
        let i=b.inner(area).inner(Margin::new(1,1));
        let mut lines=vec![Line::from(dp(&format!(" {} ({}个实例)",name,related.len()))),Line::from(dm("──────────────────────────────────────────────────────────────"))];
        if related.is_empty(){lines.push(Line::from(dm("  未检测到实例。r:重新扫描")));}
        else{for d in related.iter().take(12){let port=d.port.map(|p|format!(":{}",p)).unwrap_or_default();let pid=d.pid.map(|p|format!("pid:{}",p)).unwrap_or_default();let cfg=d.config_path.as_deref().unwrap_or("-");let log=d.log_path.as_deref().unwrap_or("-");let ver=d.version.as_deref().unwrap_or("");let icon=match d.status{ServiceStatus::Running=>"●",_=>"○"};let ic=match d.status{ServiceStatus::Running=>t.success,_=>t.error};lines.push(Line::from(vec![Span::styled(icon,Style::default().fg(ic).bold()),dm(" "),dt(&format!("{}{}",d.name,port)),dm("  "),dt(&pid)]));lines.push(Line::from(vec![dm("  cfg:"),dt(&trunc(cfg,50)),dm("  log:"),dt(&trunc(log,40)),dm("  ver:"),dt(&trunc(ver,30))]));}}
        f.render_widget(Paragraph::new(lines),i);
    }

    fn render_palette(&mut self,f:&mut Frame,t:&Theme){let a=centered_rect(60,60,f.area());f.render_widget(Clear,a);let b=Block::default().title("快速搜索").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.primary)).style(Style::default().bg(t.surface_alt));f.render_widget(b.clone(),a);let i=b.inner(a).inner(Margin::new(1,1));let[q,l]=Layout::vertical([Constraint::Length(2),Constraint::Min(1)]).areas(i);f.render_widget(Paragraph::new(format!(">{}▍",self.palette_query)),q);if self.palette_matches.is_empty()&&!self.palette_query.is_empty(){f.render_widget(Paragraph::new("无匹配"),l);}else{let items:Vec<ListItem>=self.palette_matches.iter().enumerate().map(|(i,(pi,ai))|{let s=if i==self.palette_state.selected().unwrap_or(0){Style::default().fg(t.primary).bg(t.surface_alt)}else{Style::default().fg(t.text)};ListItem::new(format!("{} > {}",self.plugins[*pi].name,self.plugins[*pi].actions[*ai].name)).style(s)}).collect();f.render_stateful_widget(List::new(items),l,&mut self.palette_state);}}
    fn render_help(&self,f:&mut Frame,t:&Theme){let a=centered_rect(44,48,f.area());f.render_widget(Clear,a);let b=Block::default().title("帮助").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.accent)).style(Style::default().bg(t.surface_alt));f.render_widget(b.clone(),a);let i=b.inner(a).inner(Margin::new(1,1));f.render_widget(Paragraph::new(vec![Line::from(Span::styled("快捷键",Style::default().fg(t.primary).bold())),Line::from(""),Line::from(vec![Span::styled("←→  ",Style::default().fg(t.accent)),Span::raw("仪表盘/插件")]),Line::from(vec![Span::styled("↑↓  ",Style::default().fg(t.accent)),Span::raw("选择")]),Line::from(vec![Span::styled("Enter",Style::default().fg(t.accent)),Span::raw("进入/执行")]),Line::from(vec![Span::styled("打字 ",Style::default().fg(t.accent)),Span::raw("输入/过滤")]),Line::from(vec![Span::styled("Back ",Style::default().fg(t.accent)),Span::raw("删除")]),Line::from(vec![Span::styled("/    ",Style::default().fg(t.accent)),Span::raw("搜索")]),Line::from(vec![Span::styled("Esc  ",Style::default().fg(t.accent)),Span::raw("返回/退出")]),Line::from(vec![Span::styled("q    ",Style::default().fg(t.accent)),Span::raw("退出")])]),i);}
    fn handle_palette(&mut self,k:crossterm::event::KeyEvent){match k.code{KeyCode::Esc=>self.mode=Mode::Normal,KeyCode::Up=>{self.pal_prev();}KeyCode::Down=>{self.pal_next();}KeyCode::Enter=>{if let Some(s)=self.palette_state.selected(){if let Some((pi,ai))=self.palette_matches.get(s){self.plugin_idx=*pi;self.action_idx=*ai;self.mode=Mode::Normal;self.view=View::Plugins;self.execute();}}}KeyCode::Backspace|KeyCode::Delete=>{self.palette_query.pop();self.update_palette();}KeyCode::Char('h') if k.modifiers.contains(KeyModifiers::CONTROL)=>{self.palette_query.pop();self.update_palette();}KeyCode::Char('w') if k.modifiers.contains(KeyModifiers::CONTROL)=>{let mut w:Vec<&str>=self.palette_query.split_whitespace().collect();w.pop();self.palette_query=w.join(" ");self.update_palette();}KeyCode::Char(c)=>{self.palette_query.push(c);self.update_palette();}_=>{}}}
    fn open_palette(&mut self){self.palette_query.clear();self.update_palette();self.mode=Mode::Palette;}
    fn update_palette(&mut self){let q=self.palette_query.to_lowercase();self.palette_matches.clear();for(pi,p)in self.plugins.iter().enumerate(){for(ai,a)in p.actions.iter().enumerate(){if q.is_empty()||format!("{} {}",p.name,a.name).to_lowercase().contains(&q){self.palette_matches.push((pi,ai));}}}if !self.palette_matches.is_empty(){self.palette_state.select(Some(0));}}
    fn pal_next(&mut self){let i=self.palette_state.selected().map_or(0,|i|(i+1).min(self.palette_matches.len().saturating_sub(1)));self.palette_state.select(Some(i));}
    fn pal_prev(&mut self){let i=self.palette_state.selected().map_or(0,|i|i.saturating_sub(1));self.palette_state.select(Some(i));}
}

fn cat_tag(c:&devtool_core::types::PluginCategory)->&str{match c{devtool_core::types::PluginCategory::DataTool=>"D",devtool_core::types::PluginCategory::SystemTool=>"S",devtool_core::types::PluginCategory::Security=>"K",devtool_core::types::PluginCategory::Middleware=>"M",devtool_core::types::PluginCategory::Script=>"R",devtool_core::types::PluginCategory::Network=>"N",devtool_core::types::PluginCategory::Custom(_)=>"?"}}
fn centered_rect(px:u16,py:u16,r:Rect)->Rect{let p=Layout::vertical([Constraint::Percentage((100-py)/2),Constraint::Percentage(py),Constraint::Percentage((100-py)/2)]).split(r);Layout::horizontal([Constraint::Percentage((100-px)/2),Constraint::Percentage(px),Constraint::Percentage((100-px)/2)]).split(p[1])[1]}
fn fmt_uptime(s:u64)->String{let d=s/86400;format!("{}d{}h",d,(s%86400)/3600)}
fn fmt_big(n:u64)->String{if n>1e9 as u64{format!("{:.1}B",n as f64/1e9)}else if n>1e6 as u64{format!("{:.1}M",n as f64/1e6)}else if n>1e3 as u64{format!("{:.1}K",n as f64/1e3)}else{n.to_string()}}
fn fmt_bytes(f:f64)->String{if f>1e9{format!("{:.2}GB",f/1e9)}else if f>1e6{format!("{:.1}MB",f/1e6)}else if f>1e3{format!("{:.0}KB",f/1e3)}else{format!("{}B",f as u64)}}
fn fmt_rate(f:f64)->String{if f>1e9{format!("{:.2}GB/s",f/1e9)}else if f>1e6{format!("{:.1}MB/s",f/1e6)}else if f>1e3{format!("{:.0}KB/s",f/1e3)}else{format!("{}B/s",f as u64)}}
fn fmt_kb(kb:u64)->String{if kb>1048576{format!("{:.2}GB",kb as f64/1048576.0)}else if kb>1024{format!("{:.1}MB",kb as f64/1024.0)}else{format!("{}KB",kb)}}
fn fmt_mem(mb:u64)->String{if mb>=1024{format!("{:.2}GB",mb as f64/1024.0)}else{format!("{}MB",mb)}}
fn trunc(s:&str,n:usize)->String{let cs:Vec<char>=s.chars().collect();if cs.len()<=n{s.into()}else{format!("{}…",cs.iter().take(n-1).collect::<String>())}}
