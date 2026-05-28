pub mod error;
pub mod manager;
pub mod plugin;
pub mod types;
#[cfg(test)] mod tests;

pub use error::{PluginError, PluginResult};
pub use manager::PluginManager;
pub use plugin::{Plugin, PluginFactory, PLUGIN_ENTRY_SYMBOL};
pub use types::*;
