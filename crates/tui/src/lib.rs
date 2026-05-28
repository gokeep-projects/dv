pub mod app;
pub mod dashboard;
pub mod middleware;
pub mod theme;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture}, execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use devtool_core::manager::PluginManager;
use ratatui::backend::CrosstermBackend;
use std::io;

pub async fn run(manager: PluginManager) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;
    let app = app::App::new(manager);
    let res = app.run(&mut terminal);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    res
}
