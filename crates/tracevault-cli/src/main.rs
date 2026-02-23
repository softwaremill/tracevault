use clap::Parser;

#[derive(Parser)]
#[command(name = "tracevault", about = "AI code governance platform")]
enum Cli {
    /// Initialize TraceVault in current repository
    Init,
    /// Show current session status
    Status,
}

fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Init => println!("tracevault init - not yet implemented"),
        Cli::Status => println!("tracevault status - not yet implemented"),
    }
}
