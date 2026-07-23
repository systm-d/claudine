//! Sous-commande `update` : auto-mise à jour du binaire depuis GitHub.

use crate::selfupdate::{self, current_target, fetch_latest, is_newer, pick_asset};

/// Met à jour claudine vers la dernière release. Avec `check_only`, se contente
/// de signaler si une version plus récente existe (n'installe rien).
pub fn run_update(check_only: bool) -> Result<(), String> {
    let current = env!("CARGO_PKG_VERSION");
    println!("Version actuelle : {current}");

    let target = current_target().ok_or_else(|| {
        format!(
            "Aucun binaire pré-construit pour cette plateforme ({}/{}). \
             Mettez à jour via votre gestionnaire de paquets ou depuis les sources.",
            std::env::consts::OS,
            std::env::consts::ARCH,
        )
    })?;

    println!("Recherche de la dernière version…");
    let release = fetch_latest()?;
    println!("Dernière version : {}", release.version);

    if !is_newer(current, &release.version) {
        println!("Vous êtes déjà à jour.");
        return Ok(());
    }

    if check_only {
        println!(
            "Une mise à jour est disponible : {current} → {}. \
             Lancez `claudine update` pour l'installer.",
            release.version
        );
        return Ok(());
    }

    let asset = pick_asset(&release.assets, &target).ok_or_else(|| {
        format!(
            "L'asset « {} » est absent de la release {}.",
            target.asset, release.tag
        )
    })?;

    println!("Téléchargement de {}…", asset.name);
    let bytes = selfupdate::download(&asset.url)?;

    println!("Installation…");
    selfupdate::install(&bytes, &target)?;

    println!(
        "Mise à jour installée : {current} → {}. Relancez claudine.",
        release.version
    );
    Ok(())
}
