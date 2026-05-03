use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tussle", version, about = "macOS hotkey conflict resolver")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan all hotkey sources and print discovered bindings.
    Scan,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan => {
            // TODO: invoke tussle_core::sources::symbolichotkeys::scan and
            // render the results.
            println!("scan: not yet implemented");
        }
    }
}
