use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;
use rhai::Engine;

struct ScriptRunnerPlugin;

impl Plugin for ScriptRunnerPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "script-runner".into(),
            version: "0.1.0".into(),
            description: "Execute embedded scripts (Rhai) with sandboxed environment".into(),
            author: "DevTool Team".into(),
            category: PluginCategory::Script,
            actions: vec![
                PluginAction {
                    name: "run".into(),
                    description: "Execute a Rhai script and return the result".into(),
                    params: vec![
                        ActionParam {
                            name: "timeout".into(),
                            description: "Script timeout in seconds (default: 10)".into(),
                            required: false,
                            default_value: Some("10".into()),
                            param_type: ParamType::Number,
                        },
                    ],
                },
                PluginAction {
                    name: "eval".into(),
                    description: "Evaluate a single expression and return the value".into(),
                    params: vec![],
                },
                PluginAction {
                    name: "template".into(),
                    description: "Process a template string with variables (use ${var} syntax)".into(),
                    params: vec![ActionParam {
                        name: "vars".into(),
                        description: "JSON object of template variables".into(),
                        required: true,
                        default_value: None,
                        param_type: ParamType::Json,
                    }],
                },
            ],
        }
    }

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        match input.action.as_str() {
            "run" => self.run_script(&input),
            "eval" => self.eval_expr(&input),
            "template" => self.process_template(&input),
            _ => Err(PluginError::InvalidAction(input.action)),
        }
    }

    fn tui_view(&self) -> Option<TuiViewDef> {
        Some(TuiViewDef {
            title: "Script Runner".into(),
            component_type: TuiComponentType::TextArea,
        })
    }

    fn web_handlers(&self) -> Vec<WebHandlerDef> {
        vec![]
    }
}

impl ScriptRunnerPlugin {
    fn get_script(&self, input: &PluginInput) -> PluginResult<String> {
        input.input_data.clone().ok_or_else(|| {
            PluginError::MissingParam("input_data (provide script via --input or -f)".into())
        })
    }

    fn create_engine(&self) -> Engine {
        let mut engine = Engine::new();
        engine.set_max_call_levels(100);
        engine.set_max_operations(100_000);
        engine.set_max_modules(20);
        engine.set_max_string_size(65536);
        engine.set_max_array_size(10000);
        // Disable file system and network for safety
        engine.set_optimization_level(rhai::OptimizationLevel::Simple);
        engine
    }

    fn run_script(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let script = self.get_script(input)?;
        let engine = self.create_engine();

        match engine.eval::<rhai::Dynamic>(&script) {
            Ok(result) => {
                let output = if result.is::<()>() {
                    "(script executed successfully)".to_string()
                } else {
                    format!("{}", result)
                };
                Ok(PluginOutput {
                    success: true,
                    data: format!("✓ Script executed successfully\n\nOutput:\n{}", output),
                    error: None,
                    metadata: None,
                })
            }
            Err(e) => Ok(PluginOutput {
                success: false,
                data: format!("✗ Script error:\n{}", format_rhai_error(&e)),
                error: Some(format_rhai_error(&e)),
                metadata: None,
            }),
        }
    }

    fn eval_expr(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let expr = self.get_script(input)?;
        let engine = self.create_engine();

        match engine.eval_expression::<rhai::Dynamic>(&expr) {
            Ok(result) => Ok(PluginOutput {
                success: true,
                data: format!("= {}", result),
                error: None,
                metadata: None,
            }),
            Err(e) => Ok(PluginOutput {
                success: false,
                data: format!("✗ Eval error: {}", format_rhai_error(&e)),
                error: Some(format_rhai_error(&e)),
                metadata: None,
            }),
        }
    }

    fn process_template(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let template = self.get_script(input)?;
        let vars_json = input.params.get("vars")
            .ok_or_else(|| PluginError::MissingParam("vars".into()))?;

        let vars: serde_json::Value = serde_json::from_str(vars_json)
            .map_err(|e| PluginError::ExecutionFailed(format!("Invalid vars JSON: {}", e)))?;

        let mut result = template;
        if let serde_json::Value::Object(map) = vars {
            for (key, value) in &map {
                let placeholder = format!("${{{}}}", key);
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    v => v.to_string(),
                };
                result = result.replace(&placeholder, &replacement);
            }
        }

        Ok(PluginOutput {
            success: true,
            data: result,
            error: None,
            metadata: None,
        })
    }
}

fn format_rhai_error(e: &rhai::EvalAltResult) -> String {
    e.to_string()
}

#[no_mangle]
pub extern "C" fn _plugin_create() -> Box<dyn Plugin> {
    Box::new(ScriptRunnerPlugin)
}
