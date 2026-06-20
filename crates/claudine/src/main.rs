mod cli;
mod tui;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "claudine", about = "Navigateur/gestionnaire des données Claude Code")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Exporte ~/.claude dans un bundle .tar.gz
    Export {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        no_history: bool,
    },
    /// Importe un bundle (avec remap des chemins)
    Import {
        bundle: PathBuf,
        #[arg(long = "map")]
        maps: Vec<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        overwrite: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        // Invocation nue : lance la TUI interactive. Le terminal est toujours
        // restauré (y compris sur erreur/panique) avant que l'erreur ne soit
        // affichée par le bloc ci-dessous.
        None => tui::run().map_err(|e| e.to_string()),
        Some(Command::Export { out, no_history }) => cli::run_export(out, no_history),
        Some(Command::Import {
            bundle,
            maps,
            dry_run,
            overwrite,
        }) => cli::run_import(bundle, maps, dry_run, overwrite),
    };
    if let Err(e) = result {
        eprintln!("Erreur : {e}");
        std::process::exit(1);
    }
}
