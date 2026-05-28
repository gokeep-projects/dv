use devtool_core::types::Lang;

pub struct PluginInfo {
    pub key: &'static str, pub name: &'static str,
    pub name_cn: &'static str, pub desc_cn: &'static str,
    pub actions: &'static [ActionInfo],
}
pub struct ActionInfo {
    pub key: &'static str, pub name: &'static str,
    pub desc: &'static str, pub desc_cn: &'static str,
}

pub fn resolve_plugin(key: &str) -> Option<&'static str> {
    PLUGINS.iter().find(|p| p.key==key||p.name==key).map(|p|p.name)
}
pub fn resolve_action(plugin: &str, key: &str) -> Option<&'static str> {
    PLUGINS.iter().find(|p|p.name==plugin).and_then(|p|p.actions.iter().find(|a|a.key==key||a.name==key)).map(|a|a.name)
}
pub fn describe(plugin: &str, action: &str, lang: Lang) -> String {
    let p = PLUGINS.iter().find(|p|p.name==plugin);
    let cn = matches!(lang, Lang::Zh);
    let pd = p.map(|p| if cn {p.desc_cn} else {p.name}).unwrap_or(plugin);
    let ad = p.and_then(|p|p.actions.iter().find(|a|a.name==action)).map(|a| if cn {a.desc_cn} else {a.desc});
    ad.map(|d| format!("{} > {}", pd, d)).unwrap_or_else(|| pd.into())
}

pub const PLUGINS: &[PluginInfo] = &[
    PluginInfo { key: "1", name: "crypto", name_cn: "加解密", desc_cn: "Base64/Hex编解码,SHA/MD5哈希,HMAC,URL编解码", actions: &[
    ActionInfo{key:"1",name:"base64-encode",desc:"Base64 encode",desc_cn:"Base64编码"},
    ActionInfo{key:"2",name:"base64-decode",desc:"Base64 decode",desc_cn:"Base64解码"},
    ActionInfo{key:"3",name:"hex-encode",desc:"Hex encode",desc_cn:"十六进制编码"},
    ActionInfo{key:"4",name:"hex-decode",desc:"Hex decode",desc_cn:"十六进制解码"},
    ActionInfo{key:"5",name:"hash",desc:"Hash",desc_cn:"哈希计算"},
    ActionInfo{key:"6",name:"hmac",desc:"HMAC",desc_cn:"HMAC签名"},
    ActionInfo{key:"7",name:"url-encode",desc:"URL encode",desc_cn:"URL编码"},
    ActionInfo{key:"8",name:"url-decode",desc:"URL decode",desc_cn:"URL解码"},
    ]},
    PluginInfo { key: "2", name: "json-tool", name_cn: "JSON工具", desc_cn: "格式化,验证,查询,差异对比,YAML/TOML转换", actions: &[
    ActionInfo{key:"1",name:"format",desc:"Format JSON",desc_cn:"JSON格式化"},
    ActionInfo{key:"2",name:"validate",desc:"Validate JSON",desc_cn:"JSON验证"},
    ActionInfo{key:"3",name:"query",desc:"Query with path",desc_cn:"路径查询"},
    ActionInfo{key:"4",name:"diff",desc:"Diff two JSON",desc_cn:"差异对比"},
    ActionInfo{key:"5",name:"minify",desc:"Minify JSON",desc_cn:"JSON压缩"},
    ActionInfo{key:"6",name:"to-yaml",desc:"To YAML",desc_cn:"转YAML"},
    ActionInfo{key:"7",name:"to-toml",desc:"To TOML",desc_cn:"转TOML"},
    ]},
    PluginInfo { key: "3", name: "terminal", name_cn: "终端", desc_cn: "Shell命令执行,环境变量,which查找", actions: &[
    ActionInfo{key:"1",name:"exec",desc:"Execute command",desc_cn:"执行命令"},
    ActionInfo{key:"2",name:"env",desc:"Show env vars",desc_cn:"显示环境变量"},
    ActionInfo{key:"3",name:"which",desc:"Locate command",desc_cn:"查找命令路径"},
    ]},
    PluginInfo { key: "4", name: "log-search", name_cn: "日志检索", desc_cn: "正则搜索,tail,IP/URL/JSON提取,统计", actions: &[
    ActionInfo{key:"1",name:"grep",desc:"Regex grep",desc_cn:"正则搜索"},
    ActionInfo{key:"2",name:"tail",desc:"Tail N lines",desc_cn:"查看尾部行"},
    ActionInfo{key:"3",name:"extract",desc:"Extract patterns",desc_cn:"提取数据"},
    ActionInfo{key:"4",name:"stats",desc:"Log statistics",desc_cn:"日志统计"},
    ]},
    PluginInfo { key: "5", name: "service-status", name_cn: "服务状态", desc_cn: "HTTP/TCP健康检查,进程检测,DNS解析", actions: &[
    ActionInfo{key:"1",name:"http-check",desc:"HTTP health check",desc_cn:"HTTP健康检查"},
    ActionInfo{key:"2",name:"tcp-check",desc:"TCP port check",desc_cn:"TCP端口检测"},
    ActionInfo{key:"3",name:"process-check",desc:"Process check",desc_cn:"进程检测"},
    ActionInfo{key:"4",name:"dns-lookup",desc:"DNS lookup",desc_cn:"DNS解析"},
    ]},
    PluginInfo { key: "6", name: "middleware", name_cn: "中间件", desc_cn: "Redis/MySQL/Kafka/ES连接测试,端口扫描", actions: &[
    ActionInfo{key:"1",name:"redis-ping",desc:"Redis ping",desc_cn:"Redis连接测试"},
    ActionInfo{key:"2",name:"mysql-ping",desc:"MySQL ping",desc_cn:"MySQL连接测试"},
    ActionInfo{key:"3",name:"kafka-brokers",desc:"Kafka check",desc_cn:"Kafka连接测试"},
    ActionInfo{key:"4",name:"elasticsearch",desc:"ES ping",desc_cn:"ES连接测试"},
    ActionInfo{key:"5",name:"port-scan",desc:"Port scan",desc_cn:"端口扫描"},
    ]},
    PluginInfo { key: "7", name: "script-runner", name_cn: "脚本执行", desc_cn: "Rhai嵌入式脚本引擎,模板替换", actions: &[
    ActionInfo{key:"1",name:"run",desc:"Run script",desc_cn:"运行脚本"},
    ActionInfo{key:"2",name:"eval",desc:"Eval expression",desc_cn:"执行表达式"},
    ActionInfo{key:"3",name:"template",desc:"Template",desc_cn:"模板替换"},
    ]},
    PluginInfo { key: "8", name: "git-tools", name_cn: "Git工具", desc_cn: "status,log,diff,branches,show,blame", actions: &[
    ActionInfo{key:"1",name:"status",desc:"Git status",desc_cn:"工作区状态"},
    ActionInfo{key:"2",name:"log",desc:"Git log",desc_cn:"提交历史"},
    ActionInfo{key:"3",name:"diff",desc:"Git diff",desc_cn:"差异对比"},
    ActionInfo{key:"4",name:"branches",desc:"List branches",desc_cn:"分支列表"},
    ActionInfo{key:"5",name:"show",desc:"Show commit",desc_cn:"查看提交"},
    ActionInfo{key:"6",name:"blame",desc:"Git blame",desc_cn:"代码归属"},
    ]},
    PluginInfo { key: "9", name: "http-client", name_cn: "HTTP客户端", desc_cn: "GET/POST/PUT/DELETE/HEAD请求,自定义Headers", actions: &[
    ActionInfo{key:"1",name:"get",desc:"GET request",desc_cn:"GET请求"},
    ActionInfo{key:"2",name:"post",desc:"POST request",desc_cn:"POST请求"},
    ActionInfo{key:"3",name:"put",desc:"PUT request",desc_cn:"PUT请求"},
    ActionInfo{key:"4",name:"delete",desc:"DELETE request",desc_cn:"DELETE请求"},
    ActionInfo{key:"5",name:"head",desc:"HEAD request",desc_cn:"HEAD请求"},
    ]},
    PluginInfo { key: "0", name: "file-tool", name_cn: "文件工具", desc_cn: "读取,写入,列表,搜索,属性,目录树", actions: &[
    ActionInfo{key:"1",name:"read",desc:"Read file",desc_cn:"读取文件"},
    ActionInfo{key:"2",name:"write",desc:"Write file",desc_cn:"写入文件"},
    ActionInfo{key:"3",name:"list",desc:"List directory",desc_cn:"目录列表"},
    ActionInfo{key:"4",name:"search",desc:"Search in files",desc_cn:"内容搜索"},
    ActionInfo{key:"5",name:"stat",desc:"File metadata",desc_cn:"文件属性"},
    ActionInfo{key:"6",name:"tree",desc:"Directory tree",desc_cn:"目录树"},
    ]},
    PluginInfo { key: "e", name: "elasticsearch", name_cn: "ES管理", desc_cn: "集群健康,节点资源,索引,错误日志,快速查询", actions: &[
    ActionInfo{key:"1",name:"health",desc:"Cluster health",desc_cn:"集群健康状态"},
    ActionInfo{key:"2",name:"stats",desc:"Cluster stats",desc_cn:"集群统计信息"},
    ActionInfo{key:"3",name:"indices",desc:"Index list",desc_cn:"索引用量"},
    ActionInfo{key:"4",name:"nodes",desc:"Node resources",desc_cn:"节点资源"},
    ActionInfo{key:"5",name:"errors",desc:"Error logs",desc_cn:"异常日志"},
    ActionInfo{key:"6",name:"search",desc:"Quick search",desc_cn:"快速查询"},
    ActionInfo{key:"7",name:"shards",desc:"Shard alloc",desc_cn:"分片分配"},
    ]},
    PluginInfo { key: "s", name: "sysinfo", name_cn: "系统信息", desc_cn: "操作系统,CPU,内存,磁盘,网络,资源,文件描述符", actions: &[
    ActionInfo{key:"1",name:"dashboard",desc:"Full dashboard",desc_cn:"完整仪表盘"},
    ActionInfo{key:"2",name:"cpu",desc:"CPU info",desc_cn:"CPU信息"},
    ActionInfo{key:"3",name:"memory",desc:"Memory usage",desc_cn:"内存使用"},
    ActionInfo{key:"4",name:"disk",desc:"Disk usage",desc_cn:"磁盘使用"},
    ActionInfo{key:"5",name:"network",desc:"Network info",desc_cn:"网络信息"},
    ]},
];