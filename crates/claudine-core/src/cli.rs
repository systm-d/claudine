use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::commands;

/// Navigateur/gestionnaire des données Claude Code.
#[derive(Parser)]
#[command(
    name = "claudine",
    version,
    about = "Navigateur/gestionnaire des données Claude Code"
)]
pub struct Cli {
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

impl Cli {
    /// Dispatch mince — aucune logique ici. Les commandes renvoient `Result<(), String>`
    /// (messages utilisateur français inchangés) ; on les remonte en `anyhow::Error` à la
    /// frontière applicative via `anyhow::Error::msg` (convention anyhow du template, spec §5).
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            // Invocation nue : lance la TUI interactive.
            None => crate::tui::run().map_err(anyhow::Error::msg),
            Some(Command::Export {
                out,
                no_history,
                home,
            }) => commands::export::run_export(out, no_history, home).map_err(anyhow::Error::msg),
            Some(Command::Import {
                bundle,
                maps,
                dry_run,
                overwrite,
                home,
            }) => commands::import::run_import(bundle, maps, dry_run, overwrite, home)
                .map_err(anyhow::Error::msg),
            Some(Command::Homes { action }) => match action {
                None => commands::homes::run_homes().map_err(anyhow::Error::msg),
                Some(HomesAction::Add { path, label }) => {
                    commands::homes::run_homes_add(path, label).map_err(anyhow::Error::msg)
                }
                Some(HomesAction::Remove { label }) => {
                    commands::homes::run_homes_remove(label).map_err(anyhow::Error::msg)
                }
            },
        }
    }
}
