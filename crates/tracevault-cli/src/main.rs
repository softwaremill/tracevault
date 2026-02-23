use clap::Parser;
use std::env;

mod api_client;
mod commands;
mod config;

#[derive(Parser)]
#[command(name = "tracevault", about = "AI code governance platform")]
enum Cli {
    /// Initialize TraceVault in current repository
    Init,
    /// Show current session status
    Status,
    /// Handle Claude Code hook event (reads JSON from stdin)
    Hook {
        #[arg(long)]
        event: String,
    },
    /// Push collected traces to the TraceVault server
    Push,
    /// Show local session statistics
    Stats,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Init => {
            let cwd = env::current_dir().expect("Cannot determine current directory");
            match commands::init::init_in_directory(&cwd) {
                Ok(()) => {
                    println!("TraceVault initialized in {}", cwd.display());
                    println!("\nTo enable Claude Code hooks, add this to .claude/settings.json:");
                    let hooks = commands::init::claude_code_hooks_json();
                    println!("{}", serde_json::to_string_pretty(&hooks).unwrap());
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        Cli::Status => println!("tracevault status - not yet implemented"),
        Cli::Hook { event: _ } => {
            let cwd = env::current_dir().expect("Cannot determine current directory");
            if let Err(e) = commands::hook::handle_hook_from_stdin(&cwd) {
                eprintln!("Hook error: {e}");
            }
        }
        Cli::Push => {
            let cwd = env::current_dir().expect("Cannot determine current directory");
            if let Err(e) = commands::push::push_traces(&cwd).await {
                eprintln!("Push error: {e}");
            }
        }
        Cli::Stats => {
            let cwd = env::current_dir().expect("Cannot determine current directory");
            if let Err(e) = commands::stats::show_stats(&cwd) {
                eprintln!("Stats error: {e}");
            }
        }
    }
}
