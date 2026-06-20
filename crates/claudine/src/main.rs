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
    /// Exporte une home Claude dans un bundle .tar.gz
    Export {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        no_history: bool,
        /// Étiquette d'une home découverte (ex. .claude-perso) ou chemin
        #[arg(long)]
        home: Option<String>,
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
        /// Étiquette d'une home découverte (ex. .claude-perso) ou chemin
        #[arg(long)]
        home: Option<String>,
    },
    /// Liste les homes Claude découvertes (.claude, .claude-perso, …)
    Homes,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        // Invocation nue : lance la TUI interactive. Le terminal est toujours
        // restauré (y compris sur erreur/panique) avant que l'erreur ne soit
        // affichée par le bloc ci-dessous.
        None => tui::run().map_err(|e| e.to_string()),
        Some(Command::Export {
            out,
            no_history,
            home,
        }) => cli::run_export(out, no_history, home),
        Some(Command::Import {
            bundle,
            maps,
            dry_run,
            overwrite,
            home,
        }) => cli::run_import(bundle, maps, dry_run, overwrite, home),
        Some(Command::Homes) => cli::run_homes(),
    };
    if let Err(e) = result {
        eprintln!("Erreur : {e}");
        std::process::exit(1);
    }
}
