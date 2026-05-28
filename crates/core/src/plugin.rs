use crate::error::PluginResult;
use crate::types::{PluginInput, PluginMetadata, PluginOutput, TuiViewDef, WebHandlerDef};

pub trait Plugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput>;
    fn tui_view(&self) -> Option<TuiViewDef>;
    fn web_handlers(&self) -> Vec<WebHandlerDef>;

    fn init(&mut self) -> PluginResult<()> {
        Ok(())
    }

    fn shutdown(&mut self) {}
}

pub type PluginFactory = fn() -> Box<dyn Plugin>;

pub const PLUGIN_ENTRY_SYMBOL: &str = "_plugin_create";
