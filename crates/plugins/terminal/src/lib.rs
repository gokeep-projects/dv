use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;
use std::process::Command;

struct TerminalPlugin;

impl Plugin for TerminalPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "terminal".into(),
            version: "0.1.0".into(),
            description: "Execute shell commands, manage terminal sessions, and capture output".into(),
            author: "DevTool Team".into(),
            category: PluginCategory::SystemTool,
            actions: vec![
                PluginAction {
                    name: "exec".into(),
                    description: "Execute a shell command and return stdout/stderr".into(),
                    params: vec![
                        ActionParam {
                            name: "command".into(),
                            description: "Shell command to execute".into(),
                            required: true,
                            default_value: None,
                            param_type: ParamType::String,
                        },
                        ActionParam {
                            name: "timeout".into(),
                            description: "Timeout in seconds (default: 30)".into(),
                            required: false,
                            default_value: Some("30".into()),
                            param_type: ParamType::Number,
                        },
                        ActionParam {
                            name: "dir".into(),
                            description: "Working directory".into(),
                            required: false,
                            default_value: None,
                            param_type: ParamType::FilePath,
                        },
                    ],
                },
                PluginAction {
                    name: "env".into(),
                    description: "List environment variables or get a specific one".into(),
                    params: vec![ActionParam {
                        name: "var".into(),
                        description: "Specific environment variable name (optional)".into(),
                        required: false,
                        default_value: None,
                        param_type: ParamType::String,
                    }],
                },
                PluginAction {
                    name: "which".into(),
                    description: "Find the location of a command in PATH".into(),
                    params: vec![ActionParam {
                        name: "command".into(),
                        description: "Command to locate".into(),
                        required: true,
                        default_value: None,
                        param_type: ParamType::String,
                    }],
                },
            ],
        }
    }

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        match input.action.as_str() {
            "exec" => self.exec_cmd(&input),
            "env" => self.show_env(&input),
            "which" => self.which_cmd(&input),
            _ => Err(PluginError::InvalidAction(input.action)),
        }
    }

    fn tui_view(&self) -> Option<TuiViewDef> {
        Some(TuiViewDef {
            title: "Terminal".into(),
            component_type: TuiComponentType::Terminal,
        })
    }

    fn web_handlers(&self) -> Vec<WebHandlerDef> {
        vec![]
    }
}

impl TerminalPlugin {
    fn exec_cmd(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let cmd_str = input
            .params
            .get("command")
            .or_else(|| input.input_data.as_ref())
            .ok_or_else(|| PluginError::MissingParam("command".into()))?;

        let timeout: u64 = input
            .params
            .get("timeout")
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let dir = input.params.get("dir").cloned();

        #[cfg(unix)]
        let output = self.run_unix(cmd_str, dir.clone(), timeout)?;
        #[cfg(windows)]
        let output = self.run_windows(cmd_str, dir, timeout)?;

        Ok(PluginOutput {
            success: output.status.success(),
            data: format!(
                "Exit code: {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            ),
            error: if !output.status.success() {
                Some(format!("Command exited with code {:?}", output.status.code()))
            } else {
                None
            },
            metadata: None,
        })
    }

    #[cfg(unix)]
    fn run_unix(
        &self,
        cmd_str: &str,
        dir: Option<String>,
        timeout: u64,
    ) -> PluginResult<std::process::Output> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(cmd_str);
        if let Some(ref d) = dir {
            cmd.current_dir(d);
        }
        // Simple timeout via wrapping in timeout command
        if timeout > 0 {
            let mut wrapped = Command::new("timeout");
            wrapped.arg(format!("{}", timeout));
            wrapped.arg("sh");
            wrapped.arg("-c");
            wrapped.arg(cmd_str);
            if let Some(d) = dir {
                wrapped.current_dir(d);
            }
            wrapped.output().map_err(|e| PluginError::ExecutionFailed(e.to_string()))
        } else {
            cmd.output().map_err(|e| PluginError::ExecutionFailed(e.to_string()))
        }
    }

    #[cfg(windows)]
    fn run_windows(
        &self,
        cmd_str: &str,
        dir: Option<String>,
        _timeout: u64,
    ) -> PluginResult<std::process::Output> {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(cmd_str);
        if let Some(d) = dir {
            cmd.current_dir(d);
        }
        cmd.output().map_err(|e| PluginError::ExecutionFailed(e.to_string()))
    }

    fn show_env(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        if let Some(var) = input.params.get("var") {
            match std::env::var(var) {
                Ok(val) => Ok(PluginOutput {
                    success: true,
                    data: val,
                    error: None,
                    metadata: None,
                }),
                Err(_) => Ok(PluginOutput {
                    success: false,
                    data: format!("Environment variable '{}' not set", var),
                    error: Some("Not found".into()),
                    metadata: None,
                }),
            }
        } else {
            let vars: Vec<String> = std::env::vars()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            Ok(PluginOutput {
                success: true,
                data: vars.join("\n"),
                error: None,
                metadata: None,
            })
        }
    }

    fn which_cmd(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let cmd_name = input
            .params
            .get("command")
            .ok_or_else(|| PluginError::MissingParam("command".into()))?;

        let mut which_cmd = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("where");
            cmd.arg(cmd_name);
            cmd
        } else {
            let mut cmd = Command::new("which");
            cmd.arg(cmd_name);
            cmd
        };

        match which_cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if stdout.is_empty() {
                    Ok(PluginOutput {
                        success: false,
                        data: format!("Command '{}' not found in PATH", cmd_name),
                        error: Some("Not found".into()),
                        metadata: None,
                    })
                } else {
                    Ok(PluginOutput {
                        success: true,
                        data: stdout,
                        error: None,
                        metadata: None,
                    })
                }
            }
            Err(e) => Ok(PluginOutput {
                success: false,
                data: format!("Error: {}", e),
                error: Some(e.to_string()),
                metadata: None,
            }),
        }
    }
}

#[no_mangle]
pub extern "C" fn _plugin_create() -> Box<dyn Plugin> {
    Box::new(TerminalPlugin)
}
