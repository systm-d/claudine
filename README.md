# Claudine

```
╭─────────────────────────────────────────────────────────╮
│  claudine  │ 1 Projets │ 2 Mémoire │ 3 Config │ 4 Extensions │
╰─────────────────────────────────────────────────────────╯
```

**Claudine** est un outil Rust TUI/CLI pour naviguer et gérer les données locales
de [Claude Code](https://docs.anthropic.com/fr/docs/claude-code) stockées dans
`~/.claude` : sessions, mémoire (`CLAUDE.md`), configuration (`settings.json`)
et extensions (hooks, plugins, serveurs MCP).

[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue)](#licence)
[![CI](https://github.com/systm-d/claudine/actions/workflows/ci.yml/badge.svg)](https://github.com/systm-d/claudine/actions/workflows/ci.yml)
[![Pages](https://github.com/systm-d/claudine/actions/workflows/pages.yml/badge.svg)](https://github.com/systm-d/claudine/actions/workflows/pages.yml)
[![Release](https://github.com/systm-d/claudine/actions/workflows/release.yml/badge.svg)](https://github.com/systm-d/claudine/actions/workflows/release.yml)

**Site :** <https://systm-d.github.io/claudine>

---

## Fonctionnalités

- **TUI interactif** — quatre onglets (Projets, Mémoire, Config, Extensions) ;
  navigation clavier complète.
- **CLI** — sous-commandes `export`, `import`, `homes` pour les scripts et la
  CI.
- **Multi-home** — plusieurs installations Claude côte à côte (`~/.claude`,
  `~/.claude-perso`, …) ; vue agrégée repliable ou home ciblée.
- **Recherche live** — filtre sur chemin/identifiant ou contenu des sessions
  (touche `/`, puis `Tab` pour basculer).
- **Ménage & corbeille** — suppression récupérable des sessions et des projets ;
  restauration, suppression définitive ou vidage depuis le TUI.
- **Export / Import** — bundle `.tar.gz` horodaté avec manifeste ; remap des
  chemins à l'import ; exclusion automatique des secrets.
- **Extensions** — lecture des hooks, plugins et serveurs MCP ; édition des
  hooks, bascule des plugins directement dans le TUI.

---

## Sûreté

- **Corbeille récupérable** : aucune suppression définitive sans confirmation
  explicite.
- **Écriture atomique** : `settings.json` est écrit via fichier temporaire +
  `rename`, jamais à moitié.
- **Sauvegarde horodatée** : avant chaque modification de `settings.json`, une
  copie `.bak-<nanos>` est créée.
- **Garde tar-slip** : les entrées d'archive contenant `..` ou une racine
  absolue sont rejetées à l'import.
- **Exclusion des secrets à l'export** : `.credentials.json`,
  `security_warnings_state_*`, `cache/`, `telemetry/`, etc., ne sont jamais
  inclus.

---

## Installation

### Cargo (crates.io)

```sh
cargo install claudine
```

### Depuis les sources

Rust ≥ 1.85 requis.

```sh
git clone https://github.com/systm-d/claudine
cd claudine
cargo install --path crates/claudine
```

### Paquets précompilés

Chaque tag `v*` déclenche le workflow [Release](.github/workflows/release.yml)
qui publie des artefacts pour les plateformes les plus répandues :

| Plateforme            | Artefact                                          |
| --------------------- | ------------------------------------------------- |
| Windows (Microsoft)   | `claudine-windows-x86_64.exe` (+ `.zip`)          |
| macOS Apple Silicon   | `claudine-macos-aarch64.tar.gz`                   |
| Linux générique       | `claudine-linux-x86_64.tar.gz`                    |
| Debian / Ubuntu       | `claudine_<version>_amd64.deb`                    |
| Fedora / RHEL         | `claudine-<version>.x86_64.rpm`                   |
| Arch Linux            | AUR (source) — `yay -S claudine`                  |

> Les Mac Intel sont couverts par Homebrew, qui compile depuis les sources
> (pas de binaire Intel pré-compilé).

```sh
# Debian / Ubuntu
sudo dpkg -i claudine_*.deb
# Fedora / RHEL
sudo rpm -i claudine-*.rpm
```

> Arch Linux : voir la section AUR ci-dessous (installation depuis les
> sources via le `PKGBUILD`, pas de paquet pré-compilé).

#### Gestionnaires de paquets

**Arch Linux — AUR :**

```sh
yay -S claudine   # ou : paru -S claudine
```

Chaque release publie aussi un `PKGBUILD` prêt à l'emploi
([`packaging/aur/PKGBUILD`](packaging/aur/PKGBUILD)) pour une installation
manuelle depuis les sources :

```sh
curl -LO https://github.com/systm-d/claudine/releases/latest/download/PKGBUILD
makepkg -si
```

**macOS — Homebrew :**

```sh
brew tap systm-d/claudine https://github.com/systm-d/claudine
brew install claudine
```

**Windows — winget** (une fois le paquet publié sur `winget-pkgs`, cf.
[`packaging/winget`](packaging/winget/README.md)) :

```powershell
winget install claudine
```

Sinon, télécharge `claudine-windows-x86_64.exe` depuis la release et place-le
dans un dossier de ton `PATH`.

---

## Utilisation

### Lancer la TUI

```sh
claudine
```

### Raccourcis clavier (TUI)

| Touche             | Action                                                     |
|--------------------|-------------------------------------------------------------|
| `1` / `2` / `3` / `4` | Projets / Mémoire / Config / Extensions               |
| `Tab`              | Section suivante                                            |
| `↑ ↓` / `j k`     | Naviguer / défiler                                          |
| `← →`             | Changer de panneau (vue Browse)                             |
| `Enter`            | Ouvrir la session sélectionnée                              |
| `Espace`           | Replier / déplier le home courant (vue agrégée)             |
| `z`                | Tout replier / tout déplier (vue agrégée)                   |
| `/`                | Rechercher (live chemin/id · `Tab` = contenu)               |
| `d` / `Suppr`      | Envoyer en corbeille : session (Projets) ou projet (Projets)|
| `m`                | Déplacer la session vers un autre projet                    |
| `c`                | Corbeille : restaurer / supprimer définitivement / vider    |
| `Esc`              | Retour (transcript) ou quitter                              |
| `PgUp` / `PgDn`    | Défilement par page                                         |
| `Home` / `End`     | Aller au début / à la fin                                   |
| `e`                | Exporter `~/.claude` en `.tar.gz`                           |
| `i`                | Importer un bundle `.tar.gz` (aperçu puis application)      |
| `E`                | Éditer le fichier de la section dans `$EDITOR`              |
| `h`                | Homes : vue agrégée (★ Tous) ou un home précis              |
| `t`                | En agrégé : changer le home cible de Mémoire/Config         |
| `?`                | Afficher / masquer l'aide                                   |
| `q` / `Ctrl-C`     | Quitter                                                     |

Extensions : `Enter` édite les hooks, `p` (dés)active les plugins.
Config : `↑↓` champ · `Enter` éditer · `←→` option · `s` enregistrer · `r` JSON brut.

### Sous-commandes CLI

```sh
# Exporter la home Claude en bundle
claudine export --out sauvegarde.tar.gz

# Exporter sans l'historique, home spécifique
claudine export --out sauvegarde.tar.gz --no-history --home .claude-perso

# Aperçu d'un import (dry-run)
claudine import sauvegarde.tar.gz --dry-run

# Importer avec remap de chemins
claudine import sauvegarde.tar.gz --map /ancien/chemin=/nouveau/chemin --overwrite

# Lister les homes enregistrées
claudine homes

# Enregistrer une home
claudine homes add ~/.claude-perso --label perso

# Retirer une home
claudine homes remove perso
```

---

## Développement

```sh
# Compiler
cargo build --workspace

# Tests
cargo test --workspace

# Linter
cargo clippy --workspace --all-targets -- -D warnings

# Formatage (rustfmt.toml : edition 2024, max_width 100)
cargo fmt
```

> **Note :** ce projet utilise `cargo fmt` ; la CI vérifie `cargo fmt --check`.
> Exécutez `cargo fmt` avant chaque commit.

---

## Contributing

Voir [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Licence

Doublement licencié sous **MIT OR Apache-2.0**,
au choix — voir [LICENSE-MIT](LICENSE-MIT) et [LICENSE-APACHE](LICENSE-APACHE).

Copyright © 2026 Kevin Delfour / systm-d.
