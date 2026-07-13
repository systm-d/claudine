mod cli;
mod tui;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "claudine",
    version,
    about = "Navigateur/gestionnaire des données Claude Code"
)]
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
    /// Gère les homes Claude (liste / ajout / retrait)
    Homes {
        #[command(subcommand)]
        action: Option<HomesAction>,
    },
}

#[derive(Subcommand)]
enum HomesAction {
    /// Enregistre une home dans la config Claudine
    Add {
        /// Chemin du répertoire de la home (ex. ~/.claude-perso)
        path: PathBuf,
        /// Étiquette explicite (sinon dérivée du dernier composant)
        #[arg(long)]
        label: Option<String>,
    },
    /// Retire une home enregistrée de la config Claudine
    Remove {
        /// Étiquette de la home à retirer
        label: String,
    },
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
        Some(Command::Homes { action }) => match action {
            None => cli::run_homes(),
            Some(HomesAction::Add { path, label }) => cli::run_homes_add(path, label),
            Some(HomesAction::Remove { label }) => cli::run_homes_remove(label),
        },
    };
    if let Err(e) = result {
        eprintln!("Erreur : {e}");
        std::process::exit(1);
    }
}
