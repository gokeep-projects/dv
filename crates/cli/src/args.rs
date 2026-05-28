use clap::{Parser, Subcommand, ValueHint, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "devtool",
    version,
    about = "A plugin-based development toolkit",
    long_about = "High-performance, plugin-based dev toolkit with CLI, TUI, and Web interfaces.",
    disable_help_subcommand = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to plugin directory
    #[arg(short, long, default_value = "./plugins")]
    pub plugin_dir: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "warn")]
    pub log_level: String,

    /// Language: zh (Chinese) or en (English)
    #[arg(long, default_value = "zh")]
    pub lang: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch the terminal UI (TUI) with plugin management
    Tui,

    /// Start the Web UI server
    Web {
        #[arg(short, long, default_value = "8080")]
        port: u16,
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },

    /// Execute a plugin action directly
    Exec {
        /// Plugin name
        plugin: String,
        /// Action to perform
        action: String,
        /// Input data string
        #[arg(short, long)]
        input: Option<String>,
        /// Input file path
        #[arg(short = 'f', long, value_hint = ValueHint::FilePath)]
        file: Option<String>,
        /// Additional key=value parameters
        #[arg(short = 'p', long = "param", value_parser = parse_key_val)]
        params: Vec<(String, String)>,
    },

    /// List all available plugins and their actions
    List,

    /// Generate shell completion script (bash/zsh/fish)
    Completions {
        shell: Shell,
    },
}

#[derive(ValueEnum, Debug, Clone)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s.find('=')
        .ok_or_else(|| format!("Invalid key=value: no '=' found in '{}'", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}
