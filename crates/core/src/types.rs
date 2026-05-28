use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PluginCategory {
    DataTool,
    SystemTool,
    Security,
    Middleware,
    Script,
    Network,
    Custom(String),
}

impl std::fmt::Display for PluginCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DataTool => write!(f, "Data Tool"),
            Self::SystemTool => write!(f, "System Tool"),
            Self::Security => write!(f, "Security"),
            Self::Middleware => write!(f, "Middleware"),
            Self::Script => write!(f, "Script"),
            Self::Network => write!(f, "Network"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String, pub version: String,
    pub description: String, pub author: String,
    pub category: PluginCategory, pub actions: Vec<PluginAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAction {
    pub name: String, pub description: String, pub params: Vec<ActionParam>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lang { Zh, En }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionParam {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
    #[serde(rename = "type")]
    pub param_type: ParamType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParamType {
    String,
    Number,
    Boolean,
    FilePath,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInput {
    pub action: String,
    pub params: std::collections::HashMap<String, String>,
    pub input_data: Option<String>,
    pub input_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginOutput {
    pub success: bool,
    pub data: String,
    pub error: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebHandlerDef {
    pub route: String,
    pub method: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiViewDef {
    pub title: String,
    pub component_type: TuiComponentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TuiComponentType {
    Form,
    Table,
    TextArea,
    LogViewer,
    Chart,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginState {
    Loaded,
    Unloaded,
    Failed,
    Running,
}
