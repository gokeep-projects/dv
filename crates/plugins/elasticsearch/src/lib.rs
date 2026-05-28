use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;

struct ElasticsearchPlugin;

impl Plugin for ElasticsearchPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "elasticsearch".into(), version: "0.1.0".into(),
            description: "ES cluster: health, stats, indices, nodes, error logs, search".into(),
            author: "DevTool Team".into(), category: PluginCategory::Middleware,
            actions: vec![
                PluginAction { name: "health".into(), description: "Cluster health: status, nodes, shards".into(),
                    params: vec![ActionParam { name: "host".into(), description: "ES host (default: http://localhost:9200)".into(), required: false, default_value: Some("http://localhost:9200".into()), param_type: ParamType::String }] },
                PluginAction { name: "stats".into(), description: "Cluster stats: indices, docs, storage, JVM, CPU".into(),
                    params: vec![ActionParam { name: "host".into(), description: "ES host".into(), required: false, default_value: Some("http://localhost:9200".into()), param_type: ParamType::String }] },
                PluginAction { name: "indices".into(), description: "Index usage: doc count, storage size per index".into(),
                    params: vec![ActionParam { name: "host".into(), description: "ES host".into(), required: false, default_value: Some("http://localhost:9200".into()), param_type: ParamType::String }] },
                PluginAction { name: "nodes".into(), description: "Node resources: CPU, RAM, heap, disk".into(),
                    params: vec![ActionParam { name: "host".into(), description: "ES host".into(), required: false, default_value: Some("http://localhost:9200".into()), param_type: ParamType::String }] },
                PluginAction { name: "errors".into(), description: "Search for ERROR/WARN in logs (filebeat or custom index)".into(),
                    params: vec![
                        ActionParam { name: "host".into(), description: "ES host".into(), required: false, default_value: Some("http://localhost:9200".into()), param_type: ParamType::String },
                        ActionParam { name: "index".into(), description: "Log index pattern (default: filebeat-*)".into(), required: false, default_value: Some("filebeat-*".into()), param_type: ParamType::String },
                        ActionParam { name: "n".into(), description: "Max results (default: 20)".into(), required: false, default_value: Some("20".into()), param_type: ParamType::Number },
                    ] },
                PluginAction { name: "search".into(), description: "Quick search documents (input_data = query body JSON)".into(),
                    params: vec![
                        ActionParam { name: "host".into(), description: "ES host".into(), required: false, default_value: Some("http://localhost:9200".into()), param_type: ParamType::String },
                        ActionParam { name: "index".into(), description: "Index pattern".into(), required: true, default_value: None, param_type: ParamType::String },
                        ActionParam { name: "n".into(), description: "Max results (default: 10)".into(), required: false, default_value: Some("10".into()), param_type: ParamType::Number },
                    ] },
                PluginAction { name: "shards".into(), description: "Shard allocation details (_cat/shards)".into(),
                    params: vec![ActionParam { name: "host".into(), description: "ES host".into(), required: false, default_value: Some("http://localhost:9200".into()), param_type: ParamType::String }] },
            ],
        }
    }

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        match input.action.as_str() {
            "health" => rt.block_on(self.health(&input)),
            "stats" => rt.block_on(self.stats(&input)),
            "indices" => rt.block_on(self.indices(&input)),
            "nodes" => rt.block_on(self.nodes(&input)),
            "errors" => rt.block_on(self.errors(&input)),
            "search" => rt.block_on(self.search(&input)),
            "shards" => rt.block_on(self.shards(&input)),
            _ => Err(PluginError::InvalidAction(input.action)),
        }
    }
    fn tui_view(&self) -> Option<TuiViewDef> { Some(TuiViewDef { title: "Elasticsearch".into(), component_type: TuiComponentType::Table }) }
    fn web_handlers(&self) -> Vec<WebHandlerDef> { vec![] }
}

impl ElasticsearchPlugin {
    fn host(&self, i: &PluginInput) -> String { i.params.get("host").cloned().unwrap_or_else(|| "http://localhost:9200".into()) }
    fn c() -> PluginResult<reqwest::Client> { reqwest::Client::builder().timeout(std::time::Duration::from_secs(15)).build().map_err(|e| PluginError::ExecutionFailed(e.to_string())) }

    async fn get(&self, i: &PluginInput, path: &str) -> PluginResult<serde_json::Value> {
        let url = format!("{}/{}", self.host(i).trim_end_matches('/'), path);
        let resp = Self::c()?.get(&url).send().await.map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let body = resp.text().await.unwrap_or_default();
        serde_json::from_str(&body).map_err(|e| PluginError::ExecutionFailed(format!("Parse error: {}", e)))
    }

    async fn health(&self, i: &PluginInput) -> PluginResult<PluginOutput> {
        let d = self.get(i, "_cluster/health").await?;
        let st = d["status"].as_str().unwrap_or("?");
        let ok = st == "green";
        let out = format!("Cluster: {}  Status: {} {}\nNodes: {}  Data nodes: {}  Shards: {}/{} active ({:.1}%)",
            d["cluster_name"].as_str().unwrap_or("?"), if ok {"✓"} else {"✗"}, st.to_uppercase(),
            d["number_of_nodes"].as_u64().unwrap_or(0), d["number_of_data_nodes"].as_u64().unwrap_or(0),
            d["active_shards"].as_u64().unwrap_or(0), d["active_shards"].as_u64().unwrap_or(0) + d["unassigned_shards"].as_u64().unwrap_or(0),
            d["active_shards_percent_as_number"].as_f64().unwrap_or(100.0));
        Ok(PluginOutput { success: ok, data: out, error: if ok {None} else {Some(format!("Status: {}", st))}, metadata: None })
    }

    async fn stats(&self, i: &PluginInput) -> PluginResult<PluginOutput> {
        let d = self.get(i, "_cluster/stats?human").await?;
        let idx = &d["indices"]; let n = &d["nodes"];
        let out = format!("Cluster Stats\n═══════════════\nIndices: {}  Docs: {}  Storage: {}\nNodes: {}  CPU: {} cores\nHeap: {} / {}  Disk: {} total, {} free",
            idx["count"].as_u64().unwrap_or(0), idx["docs"]["count"].as_u64().unwrap_or(0), idx["store"]["size"].as_str().unwrap_or("?"),
            n["count"]["total"].as_u64().unwrap_or(0), n["os"]["allocated_processors"].as_u64().unwrap_or(0),
            n["jvm"]["heap_used"].as_str().unwrap_or("?"), n["jvm"]["heap_max"].as_str().unwrap_or("?"),
            n["fs"]["total"].as_str().unwrap_or("?"), n["fs"]["free"].as_str().unwrap_or("?"));
        Ok(PluginOutput { success: true, data: out, error: None, metadata: None })
    }

    async fn indices(&self, i: &PluginInput) -> PluginResult<PluginOutput> {
        let h = self.host(i);
        let url = format!("{}/_cat/indices?v&s=store.size:desc&h=index,health,docs.count,store.size,pri.store.size", h.trim_end_matches('/'));
        let text = Self::c()?.get(&url).send().await.map_err(|e| PluginError::ExecutionFailed(e.to_string()))?.text().await.unwrap_or_default();
        let lines: Vec<_> = text.lines().collect();
        let out = if lines.len()<=1 { "No indices".into() }
                  else { format!("ES Indices\n══════════\n{}\n\n{} indices", text.trim(), lines.len().saturating_sub(1)) };
        Ok(PluginOutput { success: true, data: out, error: None, metadata: None })
    }

    async fn nodes(&self, i: &PluginInput) -> PluginResult<PluginOutput> {
        let h = self.host(i);
        let url = format!("{}/_cat/nodes?v&h=name,ip,heap.percent,ram.percent,cpu,load_1m,node.role,disk.avail", h.trim_end_matches('/'));
        let text = Self::c()?.get(&url).send().await.map_err(|e| PluginError::ExecutionFailed(e.to_string()))?.text().await.unwrap_or_default();
        Ok(PluginOutput { success: true, data: format!("Nodes\n═════\n{}", text), error: None, metadata: None })
    }

    async fn errors(&self, i: &PluginInput) -> PluginResult<PluginOutput> {
        let idx = i.params.get("index").cloned().unwrap_or_else(|| "filebeat-*".into());
        let n: usize = i.params.get("n").and_then(|s| s.parse().ok()).unwrap_or(20);
        let h = self.host(i);
        let q = serde_json::json!({"size":n,"sort":[{"@timestamp":"desc"}],"query":{"bool":{"should":[{"match":{"message":"ERROR"}},{"match":{"message":"WARN"}},{"match":{"level":"ERROR"}},{"match":{"level":"WARN"}}]}}});
        let d: serde_json::Value = Self::c()?.post(format!("{}/{}/_search", h.trim_end_matches('/'), idx)).json(&q).send().await.map_err(|e| PluginError::ExecutionFailed(e.to_string()))?.json().await.unwrap_or_default();
        let empty_arr = vec![];
        let hits_arr = d["hits"]["hits"].as_array().unwrap_or(&empty_arr);
        if hits_arr.is_empty() { return Ok(PluginOutput { success: true, data: "No errors found".into(), error: None, metadata: None }); }
        let mut out = format!("Errors from '{}'\n══════════════\n\n", idx);
        for hit in hits_arr {
            let s = &hit["_source"]; let ts = s["@timestamp"].as_str().unwrap_or("-"); let lv = s["level"].as_str().unwrap_or("?");
            let msg = s["message"].as_str().unwrap_or("-");
            out.push_str(&format!("[{}] {} | {}\n", ts, lv, if msg.len()>180 { format!("{}...", &msg[..177]) } else { msg.into() }));
        }
        Ok(PluginOutput { success: true, data: out, error: None, metadata: None })
    }

    async fn search(&self, i: &PluginInput) -> PluginResult<PluginOutput> {
        let idx = i.params.get("index").ok_or_else(|| PluginError::MissingParam("index".into()))?;
        let n: usize = i.params.get("n").and_then(|s| s.parse().ok()).unwrap_or(10);
        let h = self.host(i);
        let q: serde_json::Value = i.input_data.as_ref().and_then(|d| serde_json::from_str(d).ok()).unwrap_or(serde_json::json!({"size":n,"query":{"match_all":{}}}));
        let d: serde_json::Value = Self::c()?.post(format!("{}/{}/_search", h.trim_end_matches('/'), idx)).json(&q).send().await.map_err(|e| PluginError::ExecutionFailed(e.to_string()))?.json().await.unwrap_or_default();
        let total = d["hits"]["total"]["value"].as_u64().unwrap_or(0);
        let empty_arr = vec![];
        let hits = d["hits"]["hits"].as_array().unwrap_or(&empty_arr);
        let mut out = format!("Search: {}  Total: {}  Shown: {}\n══════════\n\n", idx, total, hits.len());
        for hit in hits { out.push_str(&format!("── _id={} _score={:.1}\n{}\n\n", hit["_id"].as_str().unwrap_or("-"), hit["_score"].as_f64().unwrap_or(0.0), serde_json::to_string_pretty(&hit["_source"]).unwrap_or_default())); }
        Ok(PluginOutput { success: true, data: out, error: None, metadata: Some([("total".into(), total.to_string())].into_iter().collect()) })
    }

    async fn shards(&self, i: &PluginInput) -> PluginResult<PluginOutput> {
        let h = self.host(i);
        let url = format!("{}/_cat/shards?v&h=index,shard,prirep,state,docs,store,node", h.trim_end_matches('/'));
        let text = Self::c()?.get(&url).send().await.map_err(|e| PluginError::ExecutionFailed(e.to_string()))?.text().await.unwrap_or_default();
        Ok(PluginOutput { success: true, data: format!("Shards\n══════\n{}", text), error: None, metadata: None })
    }
}

#[no_mangle]
pub extern "C" fn _plugin_create() -> Box<dyn Plugin> { Box::new(ElasticsearchPlugin) }
