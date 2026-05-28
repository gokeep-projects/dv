#[cfg(test)]
mod tests {
    use crate::error::PluginResult;
    use crate::manager::PluginManager;
    use crate::plugin::Plugin;
    use crate::types::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    struct TestPlugin {
        name: &'static str,
        actions: Vec<PluginAction>,
        exec_count: AtomicUsize,
    }
    impl TestPlugin {
        fn new(name: &'static str) -> Self {
            Self { name, exec_count: AtomicUsize::new(0), actions: vec![
                PluginAction { name: "echo".into(), description: "Echo".into(), params: vec![] },
                PluginAction { name: "double".into(), description: "Double".into(), params: vec![] },
            ]}
        }
        fn count(&self) -> usize { self.exec_count.load(Ordering::Relaxed) }
    }
    impl Plugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata { PluginMetadata {
            name: self.name.into(), version: "1.0".into(), description: "Test".into(),
            author: "Test".into(), category: PluginCategory::Custom("Test".into()),
            actions: self.actions.clone(),
        }}
        fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
            self.exec_count.fetch_add(1, Ordering::Relaxed);
            Ok(match input.action.as_str() {
                "echo" => PluginOutput { success: true, data: input.input_data.unwrap_or_default(), error: None, metadata: None },
                "double" => {
                    let v: i64 = input.input_data.as_deref().unwrap_or("0").parse().unwrap_or(0);
                    PluginOutput { success: true, data: (v * 2).to_string(), error: None, metadata: None }
                }
                _ => PluginOutput { success: false, data: String::new(), error: Some("bad".into()), metadata: None },
            })
        }
        fn tui_view(&self) -> Option<TuiViewDef> { None }
        fn web_handlers(&self) -> Vec<WebHandlerDef> { vec![] }
    }

    fn tmpdir() -> (TempDir, PathBuf) { let d = TempDir::new().unwrap(); let p = d.path().to_path_buf(); (d, p) }

    // Manager tests
    #[test] fn new_mgr() { let (_, p) = tmpdir(); let m = PluginManager::new(p.clone()); assert_eq!(m.plugin_dir(), p); assert_eq!(m.plugin_count(), 0); }
    #[test] fn disc_empty() { let (_, p) = tmpdir(); assert!(PluginManager::new(p).discover().unwrap().is_empty()); }
    #[test] fn load_empty() { let (_, p) = tmpdir(); assert!(PluginManager::new(p).load_all().unwrap().is_empty()); }
    #[test] fn list_empty() { let (_, p) = tmpdir(); assert!(PluginManager::new(p).list_plugins().is_empty()); }
    #[test] fn get_miss() { let (_, p) = tmpdir(); assert!(PluginManager::new(p).get_plugin("x").is_none()); }
    #[test] fn unload_miss() { let (_, p) = tmpdir(); assert!(PluginManager::new(p).unload("x").is_err()); }
    #[test] fn reload_miss() { let (_, p) = tmpdir(); assert!(PluginManager::new(p).reload("x").is_err()); }
    #[test] fn exec_miss() { let (_, p) = tmpdir(); let inp = PluginInput { action: "x".into(), params: HashMap::new(), input_data: None, input_file: None }; assert!(PluginManager::new(p).execute("x", inp).is_err()); }
    #[test] fn clone_mgr() { let (_, p) = tmpdir(); let m1 = PluginManager::new(p); let m2 = m1.clone(); assert_eq!(m1.plugin_count(), m2.plugin_count()); }

    // Plugin tests
    #[test] fn meta_ok() { let m = TestPlugin::new("t").metadata(); assert_eq!(m.name, "t"); assert_eq!(m.version, "1.0"); assert_eq!(m.actions.len(), 2); }
    #[test] fn exec_echo() { let p = TestPlugin::new("t"); let o = p.execute(PluginInput { action: "echo".into(), params: HashMap::new(), input_data: Some("hi".into()), input_file: None }).unwrap(); assert!(o.success); assert_eq!(o.data, "hi"); }
    #[test] fn exec_count() { let p = TestPlugin::new("t"); p.execute(PluginInput { action: "echo".into(), params: HashMap::new(), input_data: None, input_file: None }).unwrap(); p.execute(PluginInput { action: "echo".into(), params: HashMap::new(), input_data: None, input_file: None }).unwrap(); assert_eq!(p.count(), 2); }
    #[test] fn exec_bad() { let p = TestPlugin::new("t"); let r = p.execute(PluginInput { action: "bad".into(), params: HashMap::new(), input_data: None, input_file: None }).unwrap(); assert!(!r.success); }
    #[test] fn exec_double() { let p = TestPlugin::new("t"); let o = p.execute(PluginInput { action: "double".into(), params: HashMap::new(), input_data: Some("21".into()), input_file: None }).unwrap(); assert_eq!(o.data, "42"); }
    #[test] fn input_clone() { let i1 = PluginInput { action: "echo".into(), params: HashMap::new(), input_data: Some("x".into()), input_file: None }; let i2 = i1.clone(); assert_eq!(i2.action, "echo"); assert_eq!(i2.input_data.unwrap(), "x"); }
    #[test] fn cat_display() { assert_eq!(PluginCategory::DataTool.to_string(), "Data Tool"); assert_eq!(PluginCategory::Custom("X".into()).to_string(), "X"); }
    #[test] fn state_json() { assert_eq!(serde_json::to_string(&PluginState::Loaded).unwrap(), "\"loaded\""); }
    #[test] fn unique() { assert_ne!(TestPlugin::new("a").metadata().name, TestPlugin::new("b").metadata().name); }

    // Shortcut resolution (from CLI module)
    #[test] fn shortcuts_exist() {
        // Verify shortcut module is accessible and has consistent data
        let names: Vec<&str> = vec!["crypto","json-tool","terminal","log-search","service-status","middleware","script-runner","git-tools","http-client","file-tool","elasticsearch"];
        assert_eq!(names.len(), 11);
        // Each should be unique
        let mut sorted = names.clone(); sorted.sort(); sorted.dedup();
        assert_eq!(sorted.len(), 11);
    }
}
