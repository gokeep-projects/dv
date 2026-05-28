use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;
use regex::Regex;
use std::fs;
use std::io::{BufRead, BufReader};

struct LogSearchPlugin;

impl Plugin for LogSearchPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "log-search".into(),
            version: "0.1.0".into(),
            description: "Intelligent log search, filtering, and analysis with regex support".into(),
            author: "DevTool Team".into(),
            category: PluginCategory::SystemTool,
            actions: vec![
                PluginAction {
                    name: "grep".into(),
                    description: "Search with regex pattern in input or file".into(),
                    params: vec![
                        ActionParam {
                            name: "pattern".into(),
                            description: "Regex pattern to search for".into(),
                            required: true,
                            default_value: None,
                            param_type: ParamType::String,
                        },
                        ActionParam {
                            name: "path".into(),
                            description: "File path to search (uses input_data if omitted)".into(),
                            required: false,
                            default_value: None,
                            param_type: ParamType::FilePath,
                        },
                        ActionParam {
                            name: "ignore-case".into(),
                            description: "Case-insensitive search (true/false)".into(),
                            required: false,
                            default_value: Some("false".into()),
                            param_type: ParamType::Boolean,
                        },
                        ActionParam {
                            name: "context".into(),
                            description: "Show N lines of context around matches".into(),
                            required: false,
                            default_value: Some("0".into()),
                            param_type: ParamType::Number,
                        },
                        ActionParam {
                            name: "invert".into(),
                            description: "Show non-matching lines instead".into(),
                            required: false,
                            default_value: Some("false".into()),
                            param_type: ParamType::Boolean,
                        },
                    ],
                },
                PluginAction {
                    name: "tail".into(),
                    description: "Return the last N lines of input or file".into(),
                    params: vec![
                        ActionParam {
                            name: "lines".into(),
                            description: "Number of lines to return (default: 50)".into(),
                            required: false,
                            default_value: Some("50".into()),
                            param_type: ParamType::Number,
                        },
                        ActionParam {
                            name: "path".into(),
                            description: "File path to tail".into(),
                            required: false,
                            default_value: None,
                            param_type: ParamType::FilePath,
                        },
                    ],
                },
                PluginAction {
                    name: "extract".into(),
                    description: "Extract structured data (JSON, timestamps, IPs, URLs) from logs".into(),
                    params: vec![ActionParam {
                        name: "type".into(),
                        description: "Data type to extract: json, ip, url, timestamp, email".into(),
                        required: true,
                        default_value: Some("json".into()),
                        param_type: ParamType::String,
                    }],
                },
                PluginAction {
                    name: "stats".into(),
                    description: "Show log statistics: line count, top patterns, error rate".into(),
                    params: vec![ActionParam {
                        name: "path".into(),
                        description: "File path to analyze".into(),
                        required: false,
                        default_value: None,
                        param_type: ParamType::FilePath,
                    }],
                },
            ],
        }
    }

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        match input.action.as_str() {
            "grep" => self.grep_search(&input),
            "tail" => self.tail_lines(&input),
            "extract" => self.extract_data(&input),
            "stats" => self.log_stats(&input),
            _ => Err(PluginError::InvalidAction(input.action)),
        }
    }

    fn tui_view(&self) -> Option<TuiViewDef> {
        Some(TuiViewDef {
            title: "Log Search".into(),
            component_type: TuiComponentType::LogViewer,
        })
    }

    fn web_handlers(&self) -> Vec<WebHandlerDef> {
        vec![]
    }
}

impl LogSearchPlugin {
    fn get_content(&self, input: &PluginInput) -> PluginResult<String> {
        if let Some(path) = input.params.get("path").or_else(|| input.input_file.as_ref()) {
            fs::read_to_string(path).map_err(|e| PluginError::ExecutionFailed(e.to_string()))
        } else if let Some(data) = &input.input_data {
            Ok(data.clone())
        } else {
            Err(PluginError::MissingParam("path or input_data".into()))
        }
    }

    fn get_lines(&self, input: &PluginInput) -> PluginResult<Vec<String>> {
        if let Some(path) = input.params.get("path").or_else(|| input.input_file.as_ref()) {
            let file = fs::File::open(path).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
            Ok(BufReader::new(file)
                .lines()
                .filter_map(|l| l.ok())
                .collect())
        } else if let Some(data) = &input.input_data {
            Ok(data.lines().map(|l| l.to_string()).collect())
        } else {
            Err(PluginError::MissingParam("path or input_data".into()))
        }
    }

    fn grep_search(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let pattern = input
            .params
            .get("pattern")
            .ok_or_else(|| PluginError::MissingParam("pattern".into()))?;

        let ignore_case = input
            .params
            .get("ignore-case")
            .map(|s| s == "true")
            .unwrap_or(false);

        let invert = input
            .params
            .get("invert")
            .map(|s| s == "true")
            .unwrap_or(false);

        let context: usize = input
            .params
            .get("context")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let mut regex_str = pattern.clone();
        if ignore_case {
            regex_str = format!("(?i){}", regex_str);
        }

        let re = Regex::new(&regex_str)
            .map_err(|e| PluginError::ExecutionFailed(format!("Invalid regex: {}", e)))?;

        let lines = self.get_lines(input)?;
        let mut result = Vec::new();
        let matched: Vec<bool> = lines.iter().map(|l| re.is_match(l)).collect();

        for (i, line) in lines.iter().enumerate() {
            let is_match = matched[i];
            let should_show = if invert { !is_match } else { is_match };

            if should_show {
                if context > 0 {
                    for offset in 1..=context {
                        if i >= offset && !matched[i - offset] && i > 0 {
                            result.push(format!(
                                "\x1b[90m{}:\x1b[0m {}",
                                i - offset + 1,
                                &lines[i - offset]
                            ));
                        }
                    }
                }

                let colored = if is_match {
                    re.replace_all(line, |caps: &regex::Captures| {
                        format!("\x1b[1;33m{}\x1b[0m", &caps[0])
                    })
                    .to_string()
                } else {
                    line.clone()
                };
                result.push(format!("\x1b[36m{}:\x1b[0m {}", i + 1, colored));

                if context > 0 {
                    for offset in 1..=context {
                        if i + offset < lines.len() && !matched[i + offset] {
                            result.push(format!(
                                "\x1b[90m{}:\x1b[0m {}",
                                i + offset + 1,
                                &lines[i + offset]
                            ));
                        }
                    }
                }
            }
        }

        let match_count = matched.iter().filter(|&&m| m).count();
        let status = if invert {
            format!(
                "Found {} non-matching lines out of {}",
                result.len(),
                lines.len()
            )
        } else {
            format!(
                "Found {} matches in {} lines",
                match_count,
                lines.len()
            )
        };

        Ok(PluginOutput {
            success: true,
            data: if result.is_empty() {
                "No results found.".into()
            } else {
                format!("{}\n\n{}", status, result.join("\n"))
            },
            error: None,
            metadata: Some(
                [("matches".into(), match_count.to_string()), ("total_lines".into(), lines.len().to_string())]
                    .into_iter()
                    .collect(),
            ),
        })
    }

    fn tail_lines(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let n: usize = input
            .params
            .get("lines")
            .and_then(|s| s.parse().ok())
            .unwrap_or(50);

        let lines = self.get_lines(input)?;
        let start = if lines.len() > n { lines.len() - n } else { 0 };
        let result = lines[start..]
            .iter()
            .enumerate()
            .map(|(i, l)| format!("{}: {}", start + i + 1, l))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(PluginOutput {
            success: true,
            data: if result.is_empty() { "(empty)".into() } else { result },
            error: None,
            metadata: None,
        })
    }

    fn extract_data(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let extract_type = input
            .params
            .get("type")
            .map(|s| s.as_str())
            .unwrap_or("json");

        let content = self.get_content(input)?;

        let pattern = match extract_type {
            "json" => r"\{(?:[^{}]|(?:\{[^{}]*\}))*\}",
            "ip" => r"\b(?:\d{1,3}\.){3}\d{1,3}\b",
            "url" => r#"https?://[^\s<>"'{}|\\^`\[\]]+"#,
            "timestamp" => r"\d{4}[-/]\d{2}[-/]\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?",
            "email" => r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b",
            _ => return Err(PluginError::InvalidAction(format!("Unknown extract type: {}", extract_type))),
        };

        let re = Regex::new(pattern).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let results: Vec<String> = re
            .find_iter(&content)
            .enumerate()
            .map(|(i, m)| format!("[{}] {}", i + 1, m.as_str()))
            .collect();

        Ok(PluginOutput {
            success: true,
            data: if results.is_empty() {
                format!("No '{}' patterns found.", extract_type)
            } else {
                format!(
                    "Extracted {} '{}' items:\n{}",
                    results.len(),
                    extract_type,
                    results.join("\n")
                )
            },
            error: None,
            metadata: None,
        })
    }

    fn log_stats(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let lines = self.get_lines(input)?;
        let total = lines.len();
        let error_count = lines
            .iter()
            .filter(|l| {
                l.to_lowercase().contains("error")
                    || l.to_lowercase().contains("fatal")
                    || l.to_lowercase().contains("panic")
            })
            .count();
        let warn_count = lines
            .iter()
            .filter(|l| l.to_lowercase().contains("warn"))
            .count();

        let result = format!(
            "Log Statistics\n\
             ===============\n\
             Total lines:    {}\n\
             Error lines:    {} ({:.1}%)\n\
             Warning lines:  {} ({:.1}%)\n\
             Healthy lines:  {} ({:.1}%)",
            total,
            error_count,
            if total > 0 { error_count as f64 / total as f64 * 100.0 } else { 0.0 },
            warn_count,
            if total > 0 { warn_count as f64 / total as f64 * 100.0 } else { 0.0 },
            total.saturating_sub(error_count + warn_count),
            if total > 0 {
                (total - error_count - warn_count) as f64 / total as f64 * 100.0
            } else {
                0.0
            }
        );

        Ok(PluginOutput {
            success: true,
            data: result,
            error: None,
            metadata: Some(
                [
                    ("error_count".into(), error_count.to_string()),
                    ("warn_count".into(), warn_count.to_string()),
                    ("total".into(), total.to_string()),
                ]
                .into_iter()
                .collect(),
            ),
        })
    }
}

#[no_mangle]
pub extern "C" fn _plugin_create() -> Box<dyn Plugin> {
    Box::new(LogSearchPlugin)
}
