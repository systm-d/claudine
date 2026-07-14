# Changelog

Toutes les modifications notables de ce projet sont documentées ici.

Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/)
Versionnage : [Semantic Versioning](https://semver.org/lang/fr/)

---

## [Unreleased]

## [0.1.0] - 2026-07-13

### Aligné sur le standard partagé (`rust-cli-template` + josephine)

- **Structure** : `claudine-core` porte désormais la CLI (`cli.rs` + `commands/*`) et
  la TUI (`tui/*`) en plus de la logique ; le binaire `claudine` est un shim.
- **Fondation** : edition 2024, MSRV 1.85, `cargo fmt` adopté (+ gate CI), `[workspace.lints]`
  (`unsafe_code = forbid`), profil release optimisé (LTO/strip), `rust-toolchain.toml`.
- **CI/CD** : `ci.yml` (fmt, clippy, matrice de test multi-OS, coverage, sécurité) ;
  `release.yml` complété (Homebrew, AUR, publication crates.io opt-in) ; `pages.yml`.
- **Packaging** : recettes AUR et Homebrew (en plus de deb/rpm/winget).
- **Site** : page d'accueil Zola (hero, fonctionnalités, installation) déployable sur
  GitHub Pages.
- **Standards** : `CONVENTIONS.md`, `CLAUDE.md`, `AGENTS.md`, `deny.toml`, `tarpaulin.toml`,
  `.cargo/audit.toml`, `dependabot.yml`, `CODEOWNERS`.

## [0.0.2] - 2026-07-13

### Général

- CLI : flag `--version` / `-V` (rapporte `claudine 0.0.2`).
- En-tête TUI : glyphe Claude officiel (remplace l'approximation précédente).

### Phase 2c — Marketplaces, catalogue et installation de plugins (juin 2026)

Ajouté :
- Gestion des marketplaces (`M`) : ajout et mise à jour via `git`, exécutés en
  tâche de fond (spinner) sans bloquer l'interface.
- Catalogue des plugins d'une marketplace : liste avec statut installé/activé.
- Installation d'un plugin depuis le catalogue (`i`) : prise en charge des quatre
  types de source (`url`, `git-subdir`, `github`, chemin relatif), copie ou clone
  en cache, écriture du registre `installed_plugins.json` et activation
  automatique.
- Désinstallation d'un plugin : suppression du cache effectuée après l'écriture
  du registre et des réglages.

Core (`claudine-core`) :
- `install_plugin` : installation durcie contre le path-traversal (validation des
  noms, confinement au cache) et le suivi de symlinks (canonicalisation des
  sources) ; nettoyage des dossiers temporaires sur tous les chemins d'erreur.
- Édition des serveurs MCP dans `settings.json` en préservant les autres réglages.

### Phase 2a — Édition des hooks et bascule des plugins (juin 2026)

Ajouté :
- Section Extensions dans la TUI : lecture et affichage des hooks, plugins et
  serveurs MCP.
- Éditeur de hooks : navigation par groupes, édition des champs et commandes,
  validation à l'enregistrement (évènement et commande non vides).
- Bascule des plugins (`p` depuis l'onglet Extensions) : toggle
  `enabledPlugins` dans `settings.json`.
- Corbeille : restauration d'une session ou d'un projet (touche `c`).
- Suppression d'un projet entier (`d` sur le panneau Projets).
- Raccourcis documentés dans l'overlay d'aide (`?`).

Core (`claudine-core`) :
- `write_hooks` : réécriture des hooks en préservant les autres réglages.
- `set_plugin_enabled` : toggle `enabledPlugins`.
- `read_hook_groups` : lecture des groupes de hooks depuis `settings.json`.

### Phase 1 — Navigation et migration (juin 2026)

Ajouté :
- TUI interactif : quatre onglets (Projets, Mémoire, Config, Extensions).
- Support multi-home : vue agrégée repliable ou home ciblée.
- Recherche live sur chemin/id et contenu des sessions (`/`).
- Sélecteur de homes (`h`).
- Déplacement de session vers un autre projet (`m`).
- Export d'une home en bundle `.tar.gz` signé d'un manifeste.
- Import avec remap des chemins, dry-run, garde tar-slip.
- Exclusion automatique des secrets à l'export (`.credentials.json`, etc.).
- Écriture atomique de `settings.json` avec sauvegarde horodatée.
- CLI : sous-commandes `export`, `import`, `homes add`, `homes remove`.
- Crate `claudine-core` : lib pure (sans UI) avec tests unitaires.

---

[Unreleased]: https://github.com/systm-d/claudine/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/systm-d/claudine/releases/tag/v0.1.0
[0.0.2]: https://github.com/systm-d/claudine/releases/tag/v0.0.2
