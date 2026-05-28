use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;
use std::process::Command;

struct GitToolsPlugin;

impl Plugin for GitToolsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "git-tools".into(),
            version: "0.1.0".into(),
            description: "Git repository tools: status, log, diff, branch management".into(),
            author: "DevTool Team".into(),
            category: PluginCategory::SystemTool,
            actions: vec![
                PluginAction {
                    name: "status".into(),
                    description: "Show git working tree status (short format)".into(),
                    params: vec![ActionParam {
                        name: "path".into(), description: "Repository path (default: current dir)".into(),
                        required: false, default_value: None, param_type: ParamType::FilePath,
                    }],
                },
                PluginAction {
                    name: "log".into(),
                    description: "Show git commit history with formatting options".into(),
                    params: vec![
                        ActionParam {
                            name: "n".into(), description: "Number of commits (default: 20)".into(),
                            required: false, default_value: Some("20".into()), param_type: ParamType::Number,
                        },
                        ActionParam {
                            name: "path".into(), description: "Repository path".into(),
                            required: false, default_value: None, param_type: ParamType::FilePath,
                        },
                        ActionParam {
                            name: "format".into(), description: "oneline, short, medium, graph".into(),
                            required: false, default_value: Some("oneline".into()), param_type: ParamType::String,
                        },
                    ],
                },
                PluginAction {
                    name: "diff".into(),
                    description: "Show working tree diff with stat summary".into(),
                    params: vec![
                        ActionParam {
                            name: "staged".into(), description: "Show staged changes (true/false)".into(),
                            required: false, default_value: Some("false".into()), param_type: ParamType::Boolean,
                        },
                        ActionParam {
                            name: "path".into(), description: "Repository path".into(),
                            required: false, default_value: None, param_type: ParamType::FilePath,
                        },
                    ],
                },
                PluginAction {
                    name: "branches".into(),
                    description: "List all branches (local and remote)".into(),
                    params: vec![ActionParam {
                        name: "path".into(), description: "Repository path".into(),
                        required: false, default_value: None, param_type: ParamType::FilePath,
                    }],
                },
                PluginAction {
                    name: "show".into(),
                    description: "Show details of a specific commit".into(),
                    params: vec![
                        ActionParam {
                            name: "commit".into(), description: "Commit hash or ref (e.g. HEAD~1)".into(),
                            required: false, default_value: Some("HEAD".into()), param_type: ParamType::String,
                        },
                        ActionParam {
                            name: "path".into(), description: "Repository path".into(),
                            required: false, default_value: None, param_type: ParamType::FilePath,
                        },
                    ],
                },
                PluginAction {
                    name: "blame".into(),
                    description: "Show line-by-line authorship for a file".into(),
                    params: vec![
                        ActionParam {
                            name: "file".into(), description: "File path relative to repo root".into(),
                            required: true, default_value: None, param_type: ParamType::FilePath,
                        },
                        ActionParam {
                            name: "path".into(), description: "Repository path".into(),
                            required: false, default_value: None, param_type: ParamType::FilePath,
                        },
                    ],
                },
            ],
        }
    }

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        match input.action.as_str() {
            "status" => self.git_cmd("status", &["--short", "--branch"], &input),
            "log" => self.git_log(&input),
            "diff" => self.git_diff(&input),
            "branches" => self.git_cmd("branch", &["-a", "-v"], &input),
            "show" => self.git_show(&input),
            "blame" => self.git_blame(&input),
            _ => Err(PluginError::InvalidAction(input.action)),
        }
    }

    fn tui_view(&self) -> Option<TuiViewDef> {
        Some(TuiViewDef { title: "Git Tools".into(), component_type: TuiComponentType::LogViewer })
    }

    fn web_handlers(&self) -> Vec<WebHandlerDef> { vec![] }
}

impl GitToolsPlugin {
    fn repo_path<'a>(&self, input: &'a PluginInput) -> &'a str {
        input.params.get("path").map(|s| s.as_str()).unwrap_or(".")
    }

    fn git_cmd(&self, subcmd: &str, args: &[&str], input: &PluginInput) -> PluginResult<PluginOutput> {
        let cwd = self.repo_path(input);
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(cwd).arg(subcmd);
        for a in args { cmd.arg(a); }

        let output = cmd.output().map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        Ok(PluginOutput {
            success: output.status.success(),
            data: if stdout.is_empty() && !output.status.success() { stderr.clone() } else { stdout },
            error: if output.status.success() { None } else { Some(stderr) },
            metadata: None,
        })
    }

    fn git_log(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let n = input.params.get("n").and_then(|s| s.parse().ok()).unwrap_or(20);
        let format = input.params.get("format").map(|s| s.as_str()).unwrap_or("oneline");
        let cwd = self.repo_path(input);

        let fmt_arg = match format {
            "oneline" => "--oneline",
            "graph" => "--graph",
            _ => "--format=%h %an: %s",
        };

        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(cwd).arg("log").arg(format!("-n{}", n)).arg(fmt_arg);
        if format == "graph" { cmd.arg("--all"); }

        let output = cmd.output().map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        Ok(PluginOutput {
            success: true,
            data: String::from_utf8_lossy(&output.stdout).into_owned(),
            error: None,
            metadata: None,
        })
    }

    fn git_diff(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let staged = input.params.get("staged").map(|s| s == "true").unwrap_or(false);
        let cwd = self.repo_path(input);
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(cwd).arg("diff").arg("--stat").arg("--color=never");
        if staged { cmd.arg("--staged"); }

        let output = cmd.output().map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        Ok(PluginOutput {
            success: true,
            data: String::from_utf8_lossy(&output.stdout).into_owned(),
            error: None,
            metadata: None,
        })
    }

    fn git_show(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let commit = input.params.get("commit").map(|s| s.as_str()).unwrap_or("HEAD");
        let cwd = self.repo_path(input);
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(cwd).arg("show").arg("--stat").arg("--color=never").arg(commit);

        let output = cmd.output().map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        Ok(PluginOutput {
            success: output.status.success(),
            data: String::from_utf8_lossy(&output.stdout).into_owned(),
            error: if output.status.success() { None } else {
                Some(String::from_utf8_lossy(&output.stderr).into_owned())
            },
            metadata: None,
        })
    }

    fn git_blame(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let file = input.params.get("file")
            .ok_or_else(|| PluginError::MissingParam("file".into()))?;
        let cwd = self.repo_path(input);
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(cwd).arg("blame").arg("--date=short").arg(file);

        let output = cmd.output().map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        Ok(PluginOutput {
            success: output.status.success(),
            data: String::from_utf8_lossy(&output.stdout).into_owned(),
            error: if output.status.success() { None } else {
                Some(String::from_utf8_lossy(&output.stderr).into_owned())
            },
            metadata: None,
        })
    }
}

#[no_mangle]
pub extern "C" fn _plugin_create() -> Box<dyn Plugin> {
    Box::new(GitToolsPlugin)
}
