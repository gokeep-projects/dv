use crate::theme::Theme;
use ratatui::{layout::{Constraint,Layout,Margin,Rect},style::{Style,Stylize},text::{Line,Span},widgets::{Block,BorderType,Borders,List,ListItem,ListState,Paragraph,Wrap},Frame};
use super::config::{load_config,add_tomcat_conn,TomcatConn};
use std::process::Command;

#[derive(Clone)] pub struct TomcatState{pub connections:Vec<TomcatConn>,pub selected:usize,pub mode:TomcatMode,pub output:String,pub config:String,pub logs:Vec<String>,pub log_path:String,pub pid_info:String,pub java_opts:String,pub scroll:usize,pub config_name:String,pub catalina_home:String,pub log_conf:String,pub edit_field:u8}
#[derive(Clone,Copy,PartialEq)] pub enum TomcatMode{Normal,AddConn,ViewConfig,Logs}
impl Default for TomcatState{fn default()->Self{Self{connections:vec![],selected:0,mode:TomcatMode::Normal,output:String::new(),config:String::new(),logs:vec![],log_path:String::new(),pid_info:String::new(),java_opts:String::new(),scroll:0,config_name:String::new(),catalina_home:String::new(),log_conf:String::new(),edit_field:0}}}

impl TomcatState{
    pub fn refresh_conns(&mut self){self.connections=load_config().tomcat;}
    pub fn current(&self)->Option<&TomcatConn>{self.connections.get(self.selected)}
    pub fn fetch_info(&mut self){self.check_process();self.find_logs();self.detect_home();
        let ci=if self.config.is_empty(){"未加载"}else{"已加载"};
        let li=if self.logs.is_empty(){"未找到"}else{&format!("{}条",self.logs.len())};
        self.output=format!("Tomcat状态\n进程: {}\nCATALINA_HOME: {}\n配置: {}\n日志: {}",self.pid_info,self.catalina_home,ci,li);}
    fn detect_home(&mut self){if !self.catalina_home.is_empty(){return;}
        for p in &["/opt/tomcat","/usr/local/tomcat","/usr/share/tomcat","/var/lib/tomcat"]{if std::path::Path::new(p).exists(){self.catalina_home=p.to_string();return;}}
        if let Ok(out)=Command::new("sh").arg("-c").arg("ps aux|grep -i tomcat|grep -v grep|head -1").output(){
            let s=String::from_utf8_lossy(&out.stdout);
            for part in s.split_whitespace(){if part.starts_with("-Dcatalina.home="){self.catalina_home=part.split('=').nth(1).unwrap_or("").to_string();break;}}}}
    fn check_process(&mut self){if let Ok(out)=Command::new("sh").arg("-c").arg("ps aux|grep -i 'catalina\\|tomcat'|grep -v grep").output(){
        let s=String::from_utf8_lossy(&out.stdout);let count=s.lines().filter(|l|!l.is_empty()).count();self.java_opts.clear();
        for l in s.lines(){for p in l.split_whitespace(){if p.starts_with("-Xmx")||p.starts_with("-Xms"){if !self.java_opts.is_empty(){self.java_opts.push(' ');}self.java_opts.push_str(p);}}}
        self.pid_info=if count>0{format!("运行中({}进程) Java:{}",count,self.java_opts)}else{"未运行".into()};}else{self.pid_info="未知".into();}}
    pub fn load_config(&mut self){let home=self.catalina_home.clone();if home.is_empty(){self.config="CATALINA_HOME未检测到".into();return;}
        self.config=std::fs::read_to_string(format!("{}/conf/server.xml",home)).unwrap_or_else(|e|format!("读取失败:{}",e));}
    pub fn find_logs(&mut self){let mut candidates=vec![];if !self.catalina_home.is_empty(){candidates.push(format!("{}/logs/catalina.out",self.catalina_home));}
        candidates.extend_from_slice(&["/var/log/tomcat/catalina.out".into(),"/opt/tomcat/logs/catalina.out".into()]);
        for p in candidates{if std::path::Path::new(&p).exists(){self.log_path=p;break;}}self.tail_logs();}
    pub fn tail_logs(&mut self){if self.log_path.is_empty(){return;}if let Ok(out)=Command::new("tail").arg("-n").arg("50").arg(&self.log_path).output(){self.logs=String::from_utf8_lossy(&out.stdout).lines().map(|l|l.to_string()).collect();}}
    pub fn connect_new(&mut self){let conn=TomcatConn{name:if self.config_name.is_empty(){"tomcat".into()}else{self.config_name.clone()},catalina_home:if self.catalina_home.is_empty(){None}else{Some(self.catalina_home.clone())},log_path:if self.log_conf.is_empty(){None}else{Some(self.log_conf.clone())},pid_path:None};
        let name=conn.name.clone();add_tomcat_conn(conn);self.refresh_conns();self.selected=self.connections.iter().position(|c|c.name==name).unwrap_or(0);self.mode=TomcatMode::Normal;self.fetch_info();}
}

fn dm(t:&Theme,s:&str)->Span<'static> {Span::styled(s.to_string(),Style::default().fg(t.text_dim))}
fn dp(t:&Theme,s:&str)->Span<'static> {Span::styled(s.to_string(),Style::default().fg(t.primary).bold())}
fn dt(t:&Theme,s:&str)->Span<'static> {Span::styled(s.to_string(),Style::default().fg(t.text))}

pub fn render_tomcat_overview(f:&mut Frame,area:Rect,state:&mut TomcatState,t:&Theme){
    let[sidebar,main]=Layout::horizontal([Constraint::Percentage(25),Constraint::Percentage(75)]).areas(area);
    let sb=Block::default().title(dp(t,"Tomcat")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(sb.clone(),sidebar);
    let si=sb.inner(sidebar).inner(Margin::new(1,0));
    let items:Vec<ListItem>=state.connections.iter().enumerate().map(|(i,c)|{let s=if i==state.selected{Style::default().fg(t.primary).bg(t.surface_alt).bold()}else{Style::default().fg(t.text)};ListItem::new(format!(" {} {}",if i==state.selected{"▶"}else{" "},c.name)).style(s)}).collect();
    let mut ls=ListState::default();ls.select(Some(state.selected));f.render_stateful_widget(List::new(items),si,&mut ls);
    let mb=Block::default().title(dp(t,"Tomcat 状态")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(mb.clone(),main);let mi=mb.inner(main).inner(Margin::new(1,1));
    let mut lines=vec![];
    if let Some(c)=state.current(){lines.push(Line::from(dp(t,&format!("◉ {}",c.name))));lines.push(Line::from(dm(t,"──────────────────────────────────────────────")));
        for l in state.output.lines(){lines.push(Line::from(dt(t,l)));}lines.push(Line::from(""));lines.push(Line::from(dm(t,"c:server.xml  l:日志  a:添加  Enter:刷新")));}
    else{lines.push(Line::from("  无Tomcat配置\n  a: 添加新配置"));}f.render_widget(Paragraph::new(lines),mi);}

pub fn render_tomcat_config(f:&mut Frame,area:Rect,state:&TomcatState,t:&Theme){
    let b=Block::default().title(dp(t,"server.xml")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.border)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(),area);let i=b.inner(area).inner(Margin::new(1,0));
    let lines:Vec<Line>=state.config.lines().skip(state.scroll).take(40).map(|l|Line::from(dt(t,l))).collect();
    f.render_widget(Paragraph::new(lines).wrap(Wrap{trim:false}),i);}

pub fn render_tomcat_logs(f:&mut Frame,area:Rect,state:&TomcatState,t:&Theme){
    let b=Block::default().title(dp(t,&format!("catalina.out ({})",ts(&state.log_path,40)))).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.warning)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(),area);let i=b.inner(area).inner(Margin::new(1,0));
    let lines:Vec<Line>=state.logs.iter().skip(state.scroll).take(35).map(|l|{let c=if l.contains("SEVERE")||l.contains("ERROR"){t.error}else if l.contains("WARNING"){t.warning}else if l.contains("INFO"){t.text}else{t.text_dim};Line::from(Span::styled(ts(l,120),Style::default().fg(c)))}).collect();
    f.render_widget(Paragraph::new(lines),i);}

pub fn render_tomcat_add(f:&mut Frame,area:Rect,state:&mut TomcatState,t:&Theme){
    let b=Block::default().title(dp(t,"添加 Tomcat")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(t.primary)).style(Style::default().bg(t.surface));
    f.render_widget(b.clone(),area);let i=b.inner(area).inner(Margin::new(2,2));
    let fields:[(&str,&str,u8);3]=[("名称",&state.config_name,0),("CATALINA_HOME",&state.catalina_home,1),("日志路径",&state.log_conf,2)];
    let lines:Vec<Line>=fields.iter().map(|(l,v,fi)|{let(pre,post)=if state.edit_field==*fi{("▶ "," ▍")}else{("  ","")};Line::from(vec![Span::styled(format!("{}{:<14}",pre,l),if state.edit_field==*fi{Style::default().fg(t.primary).bold()}else{Style::default().fg(t.text_dim)}),Span::styled(format!("{}{}",v,post),Style::default().fg(t.text))])}).collect();
    f.render_widget(Paragraph::new(lines),i);}

fn ts(s:&str,n:usize)->String{let cs:Vec<char>=s.chars().collect();if cs.len()<=n{s.into()}else{format!("{}…",cs.iter().take(n-1).collect::<String>())}}
