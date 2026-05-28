use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;
use regex::Regex;
use std::fs;
use std::path::Path;

struct FileToolPlugin;

impl Plugin for FileToolPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "file-tool".into(),
            version: "0.1.0".into(),
            description: "File operations: read, write, list, search, stat, and tree view".into(),
            author: "DevTool Team".into(),
            category: PluginCategory::SystemTool,
            actions: vec![
                PluginAction {
                    name: "read".into(), description: "Read and display file contents".into(),
                    params: vec![
                        ActionParam { name: "path".into(), description: "File path to read".into(), required: true, default_value: None, param_type: ParamType::FilePath },
                        ActionParam { name: "lines".into(), description: "Number of lines to read (default: all)".into(), required: false, default_value: None, param_type: ParamType::Number },
                    ],
                },
                PluginAction {
                    name: "write".into(), description: "Write content to a file (input_data becomes file content)".into(),
                    params: vec![
                        ActionParam { name: "path".into(), description: "File path to write".into(), required: true, default_value: None, param_type: ParamType::FilePath },
                        ActionParam { name: "append".into(), description: "Append mode (true) or overwrite (false)".into(), required: false, default_value: Some("false".into()), param_type: ParamType::Boolean },
                    ],
                },
                PluginAction {
                    name: "list".into(), description: "List files in a directory with size and type".into(),
                    params: vec![
                        ActionParam { name: "path".into(), description: "Directory path".into(), required: false, default_value: Some(".".into()), param_type: ParamType::FilePath },
                        ActionParam { name: "pattern".into(), description: "Glob pattern filter (e.g. *.rs)".into(), required: false, default_value: None, param_type: ParamType::String },
                        ActionParam { name: "recursive".into(), description: "Recursive listing (true/false)".into(), required: false, default_value: Some("false".into()), param_type: ParamType::Boolean },
                    ],
                },
                PluginAction {
                    name: "search".into(), description: "Search for text/regex in files".into(),
                    params: vec![
                        ActionParam { name: "path".into(), description: "Directory or file to search".into(), required: false, default_value: Some(".".into()), param_type: ParamType::FilePath },
                        ActionParam { name: "pattern".into(), description: "Pattern to search for".into(), required: true, default_value: None, param_type: ParamType::String },
                        ActionParam { name: "glob".into(), description: "File glob filter (e.g. *.rs)".into(), required: false, default_value: Some("*".into()), param_type: ParamType::String },
                        ActionParam { name: "ignore-case".into(), description: "Case insensitive (true/false)".into(), required: false, default_value: Some("true".into()), param_type: ParamType::Boolean },
                    ],
                },
                PluginAction {
                    name: "stat".into(), description: "Show file/directory metadata".into(),
                    params: vec![
                        ActionParam { name: "path".into(), description: "File or directory path".into(), required: true, default_value: None, param_type: ParamType::FilePath },
                    ],
                },
                PluginAction {
                    name: "tree".into(), description: "Display directory tree structure".into(),
                    params: vec![
                        ActionParam { name: "path".into(), description: "Root directory path".into(), required: false, default_value: Some(".".into()), param_type: ParamType::FilePath },
                        ActionParam { name: "depth".into(), description: "Max depth (default: 3)".into(), required: false, default_value: Some("3".into()), param_type: ParamType::Number },
                    ],
                },
            ],
        }
    }

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        match input.action.as_str() {
            "read" => self.read_file(&input),
            "write" => self.write_file(&input),
            "list" => self.list_dir(&input),
            "search" => self.search_files(&input),
            "stat" => self.file_stat(&input),
            "tree" => self.dir_tree(&input),
            _ => Err(PluginError::InvalidAction(input.action)),
        }
    }

    fn tui_view(&self) -> Option<TuiViewDef> {
        Some(TuiViewDef { title: "File Tool".into(), component_type: TuiComponentType::Table })
    }

    fn web_handlers(&self) -> Vec<WebHandlerDef> { vec![] }
}

impl FileToolPlugin {
    fn read_file(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let path = input.params.get("path")
            .ok_or_else(|| PluginError::MissingParam("path".into()))?;
        let max_lines: Option<usize> = input.params.get("lines").and_then(|s| s.parse().ok());
        let content = fs::read_to_string(path)
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let data = if let Some(n) = max_lines {
            content.lines().take(n).collect::<Vec<_>>().join("\n")
        } else { content };
        Ok(PluginOutput { success: true, data, error: None, metadata: None })
    }

    fn write_file(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let path = input.params.get("path")
            .ok_or_else(|| PluginError::MissingParam("path".into()))?;
        let data = input.input_data.as_deref().unwrap_or("");
        let append = input.params.get("append").map(|s| s == "true").unwrap_or(false);
        if append {
            use std::io::Write;
            let mut file = fs::OpenOptions::new().append(true).create(true).open(path)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
            file.write_all(data.as_bytes()).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        } else {
            fs::write(path, data).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        }
        Ok(PluginOutput { success: true, data: format!("Written {} bytes to {}", data.len(), path), error: None, metadata: None })
    }

    fn list_dir(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let dir = input.params.get("path").map(|s| s.as_str()).unwrap_or(".");
        let pattern = input.params.get("pattern");
        let recursive = input.params.get("recursive").map(|s| s == "true").unwrap_or(false);
        let mut entries = Vec::new();
        if recursive {
            list_recursive(&mut entries, Path::new(dir), "", 10)?;
        } else {
            for entry in fs::read_dir(dir).map_err(|e| PluginError::ExecutionFailed(e.to_string()))? {
                let e = entry.map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
                let name = e.file_name().to_string_lossy().into_owned();
                let meta = e.metadata().ok();
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let kind = if meta.map(|m| m.is_dir()).unwrap_or(false) { "DIR" } else { "FILE" };
                entries.push((name, kind.to_string(), size));
            }
        }
        if let Some(pat) = pattern {
            entries.retain(|(name, _, _)| glob_match(pat, name));
        }
        let mut lines = vec![format!("Directory: {}\n{:<40} {:<8} {:>10}", dir, "Name", "Type", "Size")];
        lines.push("─".repeat(70));
        for (name, kind, size) in &entries {
            lines.push(format!("{:<40} {:<8} {:>10}", name, kind, format_size(*size)));
        }
        lines.push(format!("\n{} entries", entries.len()));
        Ok(PluginOutput { success: true, data: lines.join("\n"), error: None, metadata: None })
    }

    fn search_files(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let dir = input.params.get("path").map(|s| s.as_str()).unwrap_or(".");
        let pattern = input.params.get("pattern")
            .or_else(|| input.input_data.as_ref())
            .ok_or_else(|| PluginError::MissingParam("pattern".into()))?;
        let glob = input.params.get("glob").map(|s| s.as_str()).unwrap_or("*");
        let ignore_case = input.params.get("ignore-case").map(|s| s == "true").unwrap_or(true);
        let regex_str = if ignore_case { format!("(?i){}", pattern) } else { pattern.clone() };
        let re = Regex::new(&regex_str)
            .map_err(|e| PluginError::ExecutionFailed(format!("Invalid regex: {}", e)))?;
        let mut results = Vec::new();
        let path = Path::new(dir);
        if path.is_file() { search_in_file(path, &re, &mut results)?; }
        else { search_in_dir(path, &re, glob, &mut results)?; }
        if results.is_empty() {
            Ok(PluginOutput { success: true, data: format!("No matches for '{}'", pattern), error: None, metadata: None })
        } else {
            Ok(PluginOutput { success: true,
                data: format!("Found {} match(es):\n\n{}", results.len(), results.join("\n")),
                error: None, metadata: Some([("matches".into(), results.len().to_string())].into_iter().collect()),
            })
        }
    }

    fn file_stat(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let path = input.params.get("path")
            .ok_or_else(|| PluginError::MissingParam("path".into()))?;
        let meta = fs::metadata(path).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let kind = if meta.is_dir() { "Directory" } else if meta.is_symlink() { "Symlink" } else { "File" };
        let result = format!(
            "Path:     {}\nType:     {}\nSize:     {} bytes ({})\nReadonly: {}",
            path, kind, meta.len(), format_size(meta.len()), meta.permissions().readonly(),
        );
        Ok(PluginOutput { success: true, data: result, error: None, metadata: None })
    }

    fn dir_tree(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let dir = input.params.get("path").map(|s| s.as_str()).unwrap_or(".");
        let depth: usize = input.params.get("depth").and_then(|s| s.parse().ok()).unwrap_or(3);
        let mut lines = vec![dir.to_string()];
        build_tree(&mut lines, Path::new(dir), "", depth)?;
        Ok(PluginOutput { success: true, data: lines.join("\n"), error: None, metadata: None })
    }
}

fn list_recursive(entries: &mut Vec<(String, String, u64)>, dir: &Path, prefix: &str, max_depth: u32) -> PluginResult<()> {
    if max_depth == 0 { return Ok(()); }
    let rd = match fs::read_dir(dir) { Ok(d) => d, Err(_) => return Ok(()) };
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.metadata().ok();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let kind = if is_dir { "DIR" } else { "FILE" };
        let full_name = if prefix.is_empty() { name } else { format!("{}/{}", prefix, name) };
        let full_name_clone = full_name.clone();
        entries.push((full_name, kind.to_string(), size));
        if is_dir {
            list_recursive(entries, &entry.path(), &full_name_clone, max_depth - 1)?;
        }
    }
    Ok(())
}

fn build_tree(lines: &mut Vec<String>, dir: &Path, prefix: &str, depth: usize) -> PluginResult<()> {
    if depth == 0 { return Ok(()); }
    let mut entries: Vec<_> = match fs::read_dir(dir) { Ok(d) => d.filter_map(|e| e.ok()).collect(), Err(_) => return Ok(()) };
    entries.sort_by_key(|e| (!e.metadata().map(|m| m.is_dir()).unwrap_or(false), e.file_name()));
    let len = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == len - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let next_prefix = format!("{}{}   ", prefix, if is_last { " " } else { "│" });
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
        if is_dir {
            lines.push(format!("{}{}{}/", prefix, connector, name));
            build_tree(lines, &entry.path(), &next_prefix, depth - 1)?;
        } else {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            lines.push(format!("{}{}{} ({})", prefix, connector, name, format_size(size)));
        }
    }
    Ok(())
}

fn search_in_file(path: &Path, re: &Regex, results: &mut Vec<String>) -> PluginResult<()> {
    let content = match fs::read_to_string(path) { Ok(c) => c, Err(_) => return Ok(()) };
    for (i, line) in content.lines().enumerate() {
        if re.is_match(line) {
            let trimmed = if line.len() > 120 { format!("{}...", &line[..117]) } else { line.to_string() };
            results.push(format!("{}:{}: {}", path.display(), i + 1, trimmed));
        }
    }
    Ok(())
}

fn search_in_dir(dir: &Path, re: &Regex, glob: &str, results: &mut Vec<String>) -> PluginResult<()> {
    let rd = match fs::read_dir(dir) { Ok(d) => d, Err(_) => return Ok(()) };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let n = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
            if n.starts_with('.') || n == "target" || n == "node_modules" { continue; }
            search_in_dir(&path, re, glob, results)?;
        } else if glob_match(glob, &path.file_name().unwrap_or_default().to_string_lossy()) {
            search_in_file(&path, re, results)?;
        }
    }
    Ok(())
}

fn glob_match(pattern: &str, name: &str) -> bool {
    let pat = pattern.replace('*', ".*").replace('?', ".");
    Regex::new(&format!("^{}$", pat)).map(|r| r.is_match(name)).unwrap_or(false)
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 { size /= 1024.0; unit += 1; }
    format!("{:.1} {}", size, UNITS[unit])
}

#[no_mangle]
pub extern "C" fn _plugin_create() -> Box<dyn Plugin> {
    Box::new(FileToolPlugin)
}
