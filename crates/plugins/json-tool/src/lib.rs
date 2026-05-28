use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;
use serde_json::Value;

struct JsonToolPlugin;

impl Plugin for JsonToolPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "json-tool".into(),
            version: "0.1.0".into(),
            description: "JSON formatter, validator, query, and diff tool".into(),
            author: "DevTool Team".into(),
            category: PluginCategory::DataTool,
            actions: vec![
                PluginAction {
                    name: "format".into(),
                    description: "Pretty-print JSON with indentation".into(),
                    params: vec![ActionParam {
                        name: "indent".into(),
                        description: "Number of spaces for indentation (default: 2)".into(),
                        required: false,
                        default_value: Some("2".into()),
                        param_type: ParamType::Number,
                    }],
                },
                PluginAction {
                    name: "validate".into(),
                    description: "Check if input is valid JSON".into(),
                    params: vec![],
                },
                PluginAction {
                    name: "query".into(),
                    description: "Query JSON with jq-style path (e.g. .users[0].name)".into(),
                    params: vec![ActionParam {
                        name: "path".into(),
                        description: "JSON path to extract".into(),
                        required: true,
                        default_value: None,
                        param_type: ParamType::String,
                    }],
                },
                PluginAction {
                    name: "diff".into(),
                    description: "Compare two JSON values (provide second JSON via 'compare' param)".into(),
                    params: vec![ActionParam {
                        name: "compare".into(),
                        description: "Second JSON value to compare against".into(),
                        required: true,
                        default_value: None,
                        param_type: ParamType::Json,
                    }],
                },
                PluginAction {
                    name: "minify".into(),
                    description: "Remove all whitespace from JSON".into(),
                    params: vec![],
                },
                PluginAction {
                    name: "to-yaml".into(),
                    description: "Convert JSON to YAML".into(),
                    params: vec![],
                },
                PluginAction {
                    name: "to-toml".into(),
                    description: "Convert JSON to TOML".into(),
                    params: vec![],
                },
            ],
        }
    }

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        match input.action.as_str() {
            "format" => self.format_json(&input),
            "validate" => self.validate_json(&input),
            "query" => self.query_json(&input),
            "diff" => self.diff_json(&input),
            "minify" => self.minify_json(&input),
            "to-yaml" => self.to_yaml(&input),
            "to-toml" => self.to_toml(&input),
            _ => Err(PluginError::InvalidAction(input.action)),
        }
    }

    fn tui_view(&self) -> Option<TuiViewDef> {
        Some(TuiViewDef {
            title: "JSON Tool".into(),
            component_type: TuiComponentType::TextArea,
        })
    }

    fn web_handlers(&self) -> Vec<WebHandlerDef> {
        vec![WebHandlerDef {
            route: "/api/plugins/json-tool".into(),
            method: "POST".into(),
            description: "Execute JSON tool actions".into(),
        }]
    }
}

impl JsonToolPlugin {
    fn get_input(&self, input: &PluginInput) -> PluginResult<String> {
        input.input_data.clone().ok_or_else(|| {
            PluginError::MissingParam("input_data (provide JSON via --input or -f)".into())
        })
    }

    fn format_json(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let raw = self.get_input(input)?;
        let indent: usize = input.params.get("indent").and_then(|s| s.parse().ok()).unwrap_or(2);
        let v: Value = serde_json::from_str(&raw).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let formatted = serde_json::to_string_pretty(&v)
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        // Re-format with custom indent if not 2
        let result = if indent != 2 {
            formatted
                .lines()
                .map(|line| {
                    let trimmed = line.trim_start();
                    let depth = (line.len() - trimmed.len()) / 2;
                    " ".repeat(depth * indent) + trimmed
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            formatted
        };

        Ok(PluginOutput {
            success: true,
            data: result,
            error: None,
            metadata: None,
        })
    }

    fn validate_json(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let raw = self.get_input(input)?;
        match serde_json::from_str::<Value>(&raw) {
            Ok(_) => Ok(PluginOutput {
                success: true,
                data: "✓ Valid JSON".into(),
                error: None,
                metadata: None,
            }),
            Err(e) => Ok(PluginOutput {
                success: false,
                data: format!("✗ Invalid JSON: {}", e),
                error: Some(e.to_string()),
                metadata: None,
            }),
        }
    }

    fn query_json(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let raw = self.get_input(input)?;
        let path = input.params.get("path").ok_or_else(|| PluginError::MissingParam("path".into()))?;
        let v: Value = serde_json::from_str(&raw).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let result = query_value(&v, path);
        Ok(PluginOutput {
            success: true,
            data: serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{}", result)),
            error: None,
            metadata: None,
        })
    }

    fn diff_json(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let raw1 = self.get_input(input)?;
        let raw2 = input.params.get("compare").ok_or_else(|| PluginError::MissingParam("compare".into()))?;
        let v1: Value = serde_json::from_str(&raw1).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let v2: Value = serde_json::from_str(raw2).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let mut diff = Vec::new();
        json_diff_path(&mut diff, "$", &v1, &v2);

        if diff.is_empty() {
            Ok(PluginOutput {
                success: true,
                data: "✓ JSON values are identical".into(),
                error: None,
                metadata: None,
            })
        } else {
            Ok(PluginOutput {
                success: true,
                data: diff.join("\n"),
                error: None,
                metadata: None,
            })
        }
    }

    fn minify_json(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let raw = self.get_input(input)?;
        let v: Value = serde_json::from_str(&raw).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let minified = serde_json::to_string(&v).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        Ok(PluginOutput {
            success: true,
            data: minified,
            error: None,
            metadata: None,
        })
    }

    fn to_yaml(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let raw = self.get_input(input)?;
        let v: Value = serde_json::from_str(&raw).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let yaml = serde_yaml::to_string(&v).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        Ok(PluginOutput {
            success: true,
            data: yaml,
            error: None,
            metadata: None,
        })
    }

    fn to_toml(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let raw = self.get_input(input)?;
        let v: Value = serde_json::from_str(&raw).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        // Convert JSON to toml via intermediate serde value
        let toml_str = toml::to_string_pretty(&v).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        Ok(PluginOutput {
            success: true,
            data: toml_str,
            error: None,
            metadata: None,
        })
    }
}

fn query_value(root: &Value, path: &str) -> Value {
    let parts: Vec<&str> = path
        .trim_start_matches('$')
        .trim_start_matches('.')
        .split('.')
        .collect();

    let mut current = root;
    for part in parts {
        if part.is_empty() {
            continue;
        }
        // Handle array indexing: "name[0]" or just "name"
        if let Some(bracket_pos) = part.find('[') {
            let field = &part[..bracket_pos];
            let idx_str = &part[bracket_pos + 1..part.len() - 1];
            if !field.is_empty() {
                current = match current.get(field) {
                    Some(v) => v,
                    None => return Value::Null,
                };
            }
            if let Ok(idx) = idx_str.parse::<usize>() {
                current = match current.get(idx) {
                    Some(v) => v,
                    None => return Value::Null,
                };
            }
        } else {
            current = match current.get(part) {
                Some(v) => v,
                None => return Value::Null,
            };
        }
    }
    current.clone()
}

fn json_diff_path(diff: &mut Vec<String>, path: &str, a: &Value, b: &Value) {
    match (a, b) {
        (Value::Object(map_a), Value::Object(map_b)) => {
            let mut all_keys: Vec<&String> = map_a.keys().chain(map_b.keys()).collect();
            all_keys.sort();
            all_keys.dedup();
            for key in all_keys {
                let new_path = format!("{}.{}", path, key);
                match (map_a.get(key), map_b.get(key)) {
                    (Some(va), Some(vb)) => json_diff_path(diff, &new_path, va, vb),
                    (Some(_), None) => diff.push(format!("- {}: removed", new_path)),
                    (None, Some(v)) => diff.push(format!("+ {}: {}", new_path, v)),
                    (None, None) => {}
                }
            }
        }
        (Value::Array(arr_a), Value::Array(arr_b)) => {
            let len = arr_a.len().max(arr_b.len());
            for i in 0..len {
                let new_path = format!("{}[{}]", path, i);
                match (arr_a.get(i), arr_b.get(i)) {
                    (Some(va), Some(vb)) => json_diff_path(diff, &new_path, va, vb),
                    (Some(_), None) => diff.push(format!("- {}: removed", new_path)),
                    (None, Some(v)) => diff.push(format!("+ {}: {}", new_path, v)),
                    (None, None) => {}
                }
            }
        }
        (a, b) if a != b => {
            diff.push(format!("~ {}: {} → {}", path, a, b));
        }
        _ => {}
    }
}

#[no_mangle]
pub extern "C" fn _plugin_create() -> Box<dyn Plugin> {
    Box::new(JsonToolPlugin)
}
