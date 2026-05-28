mod args;
mod shortcuts;

use args::{Cli, Commands, Shell};
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use devtool_core::manager::PluginManager;
use devtool_core::types::{Lang, PluginInput};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

fn main() {
    let cli = Cli::parse();
    let filter = EnvFilter::builder()
        .with_default_directive(cli.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::WARN).into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).without_time().with_writer(io::stderr).init();

    let plugin_dir = PathBuf::from(&cli.plugin_dir);
    let manager = PluginManager::new(plugin_dir.clone());
    manager.load_all().ok();
    if manager.plugin_count() == 0 {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                if let Ok(entries) = std::fs::read_dir(exe_dir) {
                    for e in entries.flatten() {
                        let p = e.path();
                        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if name.starts_with("libdevtool_plugin_") && (name.ends_with(".so")||name.ends_with(".dylib")) {
                            let dest = plugin_dir.join(name);
                            if !dest.exists() {
                                std::fs::create_dir_all(&plugin_dir).ok();
                                std::fs::copy(&p, &dest).ok();
                            }
                        }
                    }
                }
            }
        }
        manager.load_all().ok();
    }
    if let Err(e) = run(cli, &manager) {
        eprintln!("\x1b[1;31merror:\x1b[0m {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli, manager: &PluginManager) -> Result<(), Box<dyn std::error::Error>> {
    let lang = match cli.lang.as_deref() { Some("en") => Lang::En, _ => Lang::Zh };
    let cn = matches!(lang, Lang::Zh);

    match cli.command {
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            match shell { Shell::Bash=>generate(clap_complete::shells::Bash,&mut cmd,name,&mut io::stdout()), Shell::Zsh=>generate(clap_complete::shells::Zsh,&mut cmd,name,&mut io::stdout()), Shell::Fish=>generate(clap_complete::shells::Fish,&mut cmd,name,&mut io::stdout()) }
        }
        Commands::Tui => { let rt = tokio::runtime::Runtime::new()?; rt.block_on(devtool_tui::run(manager.clone()))?; }
        Commands::Web { port, host } => { let rt = tokio::runtime::Runtime::new()?; rt.block_on(devtool_web::run(manager.clone(), host, port))?; }

        Commands::Exec { plugin, action, input, file, params } => {
            manager.load_all()?;
            // Resolve shortcuts
            let pname = shortcuts::resolve_plugin(&plugin).unwrap_or(&plugin);
            let aname = shortcuts::resolve_action(pname, &action).unwrap_or(&action);

            let input_data = file.as_ref().map(|p| std::fs::read_to_string(p)).transpose()?.or(input);
            let params: HashMap<String, String> = params.into_iter().collect();
            let desc = shortcuts::describe(pname, aname, lang);

            let result = manager.execute(pname, PluginInput { action: aname.into(), params, input_data, input_file: file })?;
            if result.success { println!("{}", result.data); }
            else {
                eprintln!("\x1b[1;31m{}:\x1b[0m {}", if cn {"错误"} else {"ERROR"}, result.error.as_deref().unwrap_or("?"));
                if !result.data.is_empty() { eprintln!("{}", result.data); }
            }
        }

        Commands::List => {
            manager.load_all()?;
            let plugins = manager.list_plugins();
            if plugins.is_empty() {
                if cn { println!("\x1b[33m未找到插件\x1b[0m"); }
                else { println!("\x1b[33mNo plugins found\x1b[0m"); }
            } else {
                if cn {
                    println!("\x1b[1;36m{: <4} {: <14} {: <8} {}\x1b[0m", "编号", "插件", "版本", "说明");
                    println!("\x1b[90m{}\x1b[0m", "─".repeat(70));
                } else {
                    println!("\x1b[1;36m{: <4} {: <14} {: <8} {}\x1b[0m", "KEY", "PLUGIN", "VERSION", "DESCRIPTION");
                    println!("\x1b[90m{}\x1b[0m", "─".repeat(70));
                }
                for (meta, _) in &plugins {
                    let info = shortcuts::PLUGINS.iter().find(|p| p.name == meta.name);
                    let key = info.map(|i| i.key).unwrap_or("?");
                    let label = if cn { info.map(|i| i.name_cn).unwrap_or(&meta.name) } else { &meta.name };
                    let desc = if cn { info.map(|i| i.desc_cn).unwrap_or("") } else { &meta.description };
                    println!("\x1b[1;32m{: <4}\x1b[0m \x1b[1;37m{: <14}\x1b[0m {: <8} \x1b[90m{}\x1b[0m", key, label, meta.version, desc);
                    for a in &meta.actions {
                        let ai = info.and_then(|i| i.actions.iter().find(|x| x.name == a.name));
                        let ak = ai.map(|x| x.key).unwrap_or("?");
                        let ad = if cn { ai.map(|x| x.desc_cn).unwrap_or("") } else { &a.description };
                        println!("  \x1b[33m{: <2}\x1b[0m \x1b[36m{: <16}\x1b[0m \x1b[90m{}\x1b[0m", ak, a.name, ad);
                    }
                }
                if cn { println!("\n\x1b[1;32m{}\x1b[0m 个插件  使用: devtool exec <插件编号> <功能编号>", plugins.len()); }
                else { println!("\n\x1b[1;32m{}\x1b[0m plugins  Usage: devtool exec <plugin-key> <action-key>", plugins.len()); }
            }
        }
    }
    Ok(())
}
