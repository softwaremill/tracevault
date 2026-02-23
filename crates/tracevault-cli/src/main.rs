use clap::Parser;
use std::env;

mod api_client;
mod commands;
mod config;
mod credentials;

#[derive(Parser)]
#[command(name = "tracevault", version, about = "AI code governance platform")]
enum Cli {
    /// Initialize TraceVault in current repository
    Init {
        /// TraceVault server URL for repo registration
        #[arg(long)]
        server_url: Option<String>,
    },
    /// Show current session status
    Status,
    /// Handle Claude Code hook event (reads JSON from stdin)
    Hook {
        #[arg(long)]
        event: String,
    },
    /// Push collected traces to the TraceVault server
    Push,
    /// Sync repo remote URL with the TraceVault server
    Sync,
    /// Show local session statistics
    Stats,
    /// Log in to a TraceVault server
    Login {
        /// TraceVault server URL
        #[arg(long)]
        server_url: String,
    },
    /// Log out from the TraceVault server
    Logout,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Init { server_url } => {
            let cwd = env::current_dir().expect("Cannot determine current directory");
            match commands::init::init_in_directory(&cwd, server_url.as_deref()).await {
                Ok(()) => {
                    println!("TraceVault initialized in {}", cwd.display());
                    println!("Claude Code hooks installed in .claude/settings.json");
                    println!("Git pre-push hook installed");
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
        Cli::Sync => {
            let cwd = env::current_dir().expect("Cannot determine current directory");
            if let Err(e) = commands::sync::sync_repo(&cwd).await {
                eprintln!("Sync error: {e}");
            }
        }
        Cli::Stats => {
            let cwd = env::current_dir().expect("Cannot determine current directory");
            if let Err(e) = commands::stats::show_stats(&cwd) {
                eprintln!("Stats error: {e}");
            }
        }
        Cli::Login { server_url } => {
            if let Err(e) = commands::login::login(&server_url).await {
                eprintln!("Login error: {e}");
            }
        }
        Cli::Logout => {
            if let Err(e) = commands::logout::logout().await {
                eprintln!("Logout error: {e}");
            }
        }
    }
}
