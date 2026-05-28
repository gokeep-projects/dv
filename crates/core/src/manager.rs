use crate::error::{PluginError, PluginResult};
use crate::plugin::{Plugin, PluginFactory, PLUGIN_ENTRY_SYMBOL};
use crate::types::{PluginInput, PluginMetadata, PluginOutput, PluginState};
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

struct LoadedPlugin {
    instance: Box<dyn Plugin>,
    state: PluginState,
    path: PathBuf,
    _lib: Library,
}

pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    plugin_dir: PathBuf,
}

impl PluginManager {
    pub fn new(plugin_dir: PathBuf) -> Self {
        Self { plugins: Arc::new(RwLock::new(HashMap::new())), plugin_dir }
    }

    pub fn plugin_dir(&self) -> PathBuf { self.plugin_dir.clone() }

    pub fn discover(&self) -> PluginResult<Vec<(PathBuf, PluginMetadata)>> {
        let dir = &self.plugin_dir;
        if !dir.exists() { return Ok(vec![]); }
        let mut results = Vec::new();
        for entry in std::fs::read_dir(dir)?.flatten() {
            let path = entry.path();
            if !self.is_plugin_file(&path) { continue; }
            match self.load_plugin_metadata(&path) {
                Ok(meta) => results.push((path, meta)),
                Err(e) => warn!("Cannot read {:?}: {}", path.file_name().unwrap_or_default(), e),
            }
        }
        Ok(results)
    }

    pub fn load_all(&self) -> PluginResult<Vec<PluginMetadata>> {
        let mut discovered = self.discover()?;

        // Fallback: search the directory of the current executable
        if discovered.is_empty() {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(exe_dir) = exe.parent() {
                    if exe_dir != self.plugin_dir {
                        discovered = self.discover_in_dir(exe_dir)?;
                        if !discovered.is_empty() {
                            info!("Found {} plugins in exe directory", discovered.len());
                        }
                    }
                }
            }
        }

        // Fallback: search target/release/ relative to CWD
        if discovered.is_empty() {
            let dev_dir = std::env::current_dir().unwrap_or_default().join("target/release");
            if dev_dir.exists() && dev_dir != self.plugin_dir {
                discovered = self.discover_in_dir(&dev_dir)?;
                if !discovered.is_empty() {
                    info!("Found {} plugins in target/release/", discovered.len());
                }
            }
        }

        let mut loaded = Vec::new();
        for (path, meta) in discovered {
            match self.load(&path) {
                Ok(m) => loaded.push(m),
                Err(_) => debug!("Plugin {} already loaded, skipping", meta.name),
            }
        }
        Ok(loaded)
    }

    fn discover_in_dir(&self, dir: &Path) -> PluginResult<Vec<(PathBuf, PluginMetadata)>> {
        let mut results = Vec::new();
        if !dir.exists() { return Ok(results); }
        for entry in std::fs::read_dir(dir)?.flatten() {
            let path = entry.path();
            if !self.is_plugin_file(&path) { continue; }
            match self.load_plugin_metadata(&path) {
                Ok(meta) => results.push((path, meta)),
                Err(e) => warn!("Failed to read plugin {:?}: {}", path, e),
            }
        }
        Ok(results)
    }

    pub fn load(&self, plugin_path: &Path) -> PluginResult<PluginMetadata> {
        let path = plugin_path.canonicalize()?;

        unsafe {
            let lib = Library::new(&path).map_err(|e| {
                PluginError::LoadFailed(format!("Cannot open library {:?}: {}", path, e))
            })?;

            let factory: Symbol<PluginFactory> = lib
                .get(PLUGIN_ENTRY_SYMBOL.as_bytes())
                .map_err(|e| {
                    PluginError::LoadFailed(format!(
                        "Symbol '{}' not found in {:?}: {}",
                        PLUGIN_ENTRY_SYMBOL, path, e
                    ))
                })?;

            let mut instance = factory();
            instance.init()?;
            let metadata = instance.metadata();

            // Check duplicate using the real metadata name
            if self.plugins.read().unwrap().contains_key(&metadata.name) {
                // Drop lib before returning error (instance and lib ownership here)
                drop(instance);
                drop(lib);
                return Err(PluginError::Other(format!("Plugin '{}' already loaded", metadata.name)));
            }

            let loaded = LoadedPlugin {
                _lib: lib,
                instance,
                state: PluginState::Loaded,
                path,
            };

            self.plugins
                .write()
                .unwrap()
                .insert(metadata.name.clone(), loaded);
            info!("Plugin '{}' loaded successfully", metadata.name);
            Ok(metadata)
        }
    }

    pub fn unload(&self, name: &str) -> PluginResult<()> {
        let mut plugins = self.plugins.write().unwrap();
        if let Some(mut loaded) = plugins.remove(name) {
            loaded.instance.shutdown();
            loaded.state = PluginState::Unloaded;
            info!("Plugin '{}' unloaded", name);
            Ok(())
        } else {
            Err(PluginError::NotFound(name.to_string()))
        }
    }

    pub fn reload(&self, name: &str) -> PluginResult<PluginMetadata> {
        let path = {
            let plugins = self.plugins.read().unwrap();
            plugins
                .get(name)
                .map(|p| p.path.clone())
                .ok_or_else(|| PluginError::NotFound(name.to_string()))?
        };

        self.unload(name)?;
        self.load(&path)
    }

    pub fn execute(&self, plugin_name: &str, input: PluginInput) -> PluginResult<PluginOutput> {
        let plugins = self.plugins.read().unwrap();
        let loaded = plugins.get(plugin_name)
            .ok_or_else(|| PluginError::NotFound(plugin_name.to_string()))?;
        debug!("Executing plugin '{}' action '{}'", plugin_name, input.action);
        loaded.instance.execute(input)
    }

    pub fn list_plugins(&self) -> Vec<(PluginMetadata, PluginState)> {
        self.plugins
            .read()
            .unwrap()
            .values()
            .map(|p| (p.instance.metadata(), p.state))
            .collect()
    }

    pub fn get_plugin(&self, name: &str) -> Option<PluginMetadata> {
        self.plugins
            .read()
            .unwrap()
            .get(name)
            .map(|p| p.instance.metadata())
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins.read().unwrap().len()
    }

    fn load_plugin_metadata(&self, path: &Path) -> PluginResult<PluginMetadata> {
        unsafe {
            let lib =
                Library::new(path).map_err(|e| PluginError::LoadFailed(format!("{:?}: {}", path, e)))?;

            let factory: Symbol<PluginFactory> = lib
                .get(PLUGIN_ENTRY_SYMBOL.as_bytes())
                .map_err(|e| {
                    PluginError::LoadFailed(format!(
                        "Symbol '{}' not found: {}",
                        PLUGIN_ENTRY_SYMBOL, e
                    ))
                })?;

            let instance = factory();
            Ok(instance.metadata())
        }
    }

    fn is_plugin_file(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        #[cfg(target_os = "linux")]
        {
            name.starts_with("libdevtool_plugin_") && name.ends_with(".so")
        }
        #[cfg(target_os = "macos")]
        {
            name.starts_with("libdevtool_plugin_") && (name.ends_with(".dylib") || name.ends_with(".so"))
        }
        #[cfg(target_os = "windows")]
        {
            name.starts_with("devtool_plugin_") && name.ends_with(".dll")
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            name.contains("devtool_plugin") && (name.ends_with(".so") || name.ends_with(".dylib"))
        }
    }
}

impl Clone for PluginManager {
    fn clone(&self) -> Self {
        Self {
            plugins: Arc::clone(&self.plugins),
            plugin_dir: self.plugin_dir.clone(),
        }
    }
}

unsafe impl Send for PluginManager {}
unsafe impl Sync for PluginManager {}
