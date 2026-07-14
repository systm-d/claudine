# Design — Alignement de claudine sur `rust-cli-template` + josephine

- **Date** : 2026-07-13
- **Statut** : Approuvé (phase design)
- **Auteur** : kdelfour
- **Objectif** : Réaligner claudine (claude-tui) sur le standard partagé défini par
  `_templates/cli` (le `cargo-generate` maison) et réalisé de bout en bout par
  **josephine**, sans casser le dépôt existant : conventions homogènes (edition 2024,
  `cargo fmt`, lints), CI/CD complète, packaging, et une **partie web** (site Zola →
  GitHub Pages) aujourd'hui absente. Un seul chantier : une spec → un plan → un run SDD.

---

## 1. Contexte & motivation

Le template `_templates/cli` (spec `2026-06-27-cli-template-design.md`) vise à publier les
quatre CLIs Rust maison (repolens, guardians, **claude-tui/claudine**, hinotes) au **même
niveau de qualité**. La §9 de cette spec prévoit explicitement le réalignement de claudine.

Constat après exploration :

- Le **template est partiel** : seuls les axes *fondation / topology / state* sont
  réellement construits. Les axes `ui=cli+tui`, `privileges`, `service` et les parties
  `.github/`, `packaging/`, `site/`, `deny.toml`, `tarpaulin.toml`, `CONVENTIONS.md` sont
  **spécifiés mais pas encore dans le template**.
- **josephine** est le projet où ces parties sont **réalisées**. Il sert donc de référence
  concrète pour tout ce que le template ne fournit pas encore.

Donc l'alignement puise à **deux sources** : la **fondation** vient du template (par
génération de référence), le **web / CI / packaging / standards** se calque sur josephine.

### État de claudine (point de départ)

- Workspace `claudine-core` (logique pure, sans dépendance UI) + `claudine` (binaire
  clap + ratatui/TUI). Edition **2021**, MSRV **1.74**.
- Formatage **manuel** (règle historique « jamais `cargo fmt` »).
- Déjà présent : `LICENSE-MIT`/`LICENSE-APACHE`, `README.md`, `CHANGELOG.md`,
  `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `.github/ISSUE_TEMPLATE/*`,
  `.github/PULL_REQUEST_TEMPLATE.md`, `.github/workflows/{ci,release}.yml`,
  `packaging/arch/PKGBUILD`, `packaging/winget/README.md`, release **multi-OS**
  (Linux x86_64, Windows x86_64, macOS aarch64).
- 162 tests verts, 0 warning clippy.

---

## 2. Décisions verrouillées (issues du brainstorming)

1. **In-place** : on conserve le dépôt actuel, son **historique git** et le tag **`v0.0.2`**.
   Le scaffold de référence est généré dans un dossier de staging pour diff, puis on porte
   les pièces manquantes dans le dépôt existant (approche MIGRATION.md §9).
2. **`cargo fmt` adopté** : reformatage unique de tout le code + gate `fmt --check` en CI.
   La règle mémoire « jamais `cargo fmt` » devient **obsolète** (à mettre à jour).
3. **Un seul chantier** : une spec + un plan multi-tâches + un run SDD, revue finale globale.
4. **Structure « à la lettre du template »** : `claudine-core` absorbe `cli.rs` +
   `commands/*` + `tui/*` **et** la logique ; le binaire `claudine` devient un shim de
   3 lignes. Le core gagne `clap` + `ratatui` comme dépendances. La règle mémoire
   « core sans dépendance UI » devient **obsolète**.
5. **Multi-OS conservé** (claudine gère `~/.claude`, présent sur toutes les plateformes),
   contrairement à josephine qui est Linux-only.
6. **Version post-alignement : `0.1.0`** (bump depuis `0.0.2`, changement structurel majeur).

---

## 3. Convention de fondation (générée depuis le template)

Scaffold de référence généré avec les axes de claudine
(`topology=workspace`, `ui=cli+tui`, `state=none`, `privileges=single`, `service=none`) :

```
Cargo.toml            # edition 2024, MSRV 1.85, [workspace.lints], [profile.release], [workspace.dependencies]
rust-toolchain.toml   # channel stable + components [rustfmt, clippy]
rustfmt.toml          # edition = "2024", max_width = 100
crates/claudine/src/main.rs           # fn main() -> ExitCode { claudine_core::run() }
crates/claudine/tests/cli.rs          # tests d'intégration assert_cmd
crates/claudine-core/src/lib.rs       # pub fn run() -> ExitCode ; mod cli; mod commands; ...
crates/claudine-core/src/cli.rs       # #[derive(Parser)] Cli { #[command(subcommand)] } + dispatch mince
crates/claudine-core/src/commands/mod.rs
crates/claudine-core/src/commands/<sous-cmd>.rs   # 1 fichier/sous-cmd : Args + run() (frontière IO) + logique pure testée
```

Idiomes à adopter :
- `lib.rs::run()` : `match Cli::parse().run() { Ok(()) => SUCCESS, Err(e) => { eprintln!("error: {e:#}"); FAILURE } }`.
- `cli.rs` : dispatch **mince** uniquement (aucune logique).
- `commands/<x>.rs` : `struct Args` (clap), `pub fn run(args)` = frontière IO, logique pure
  extraite en fonctions testées `#[cfg(test)]`.
- Erreurs applicatives via **`anyhow`** ; erreurs typées de la logique via **`thiserror`**
  (conservé dans le core).

---

## 4. Structure cible du dépôt

```
claudine/
├── Cargo.toml                 # edition 2024, MSRV 1.85, [workspace.lints], [profile.release], [workspace.dependencies]
├── rust-toolchain.toml        # NOUVEAU
├── rustfmt.toml               # NOUVEAU
├── deny.toml                  # NOUVEAU (cargo-deny)
├── tarpaulin.toml             # NOUVEAU (coverage)
├── .cargo/audit.toml          # NOUVEAU (miroir des ignores de deny)
├── .github/
│   ├── workflows/
│   │   ├── ci.yml             # MODIFIÉ (fmt + coverage + security + bench-smoke)
│   │   ├── release.yml        # MODIFIÉ (+ homebrew/aur render, crates.io opt-in)
│   │   └── pages.yml          # NOUVEAU (build Zola + deploy-pages)
│   ├── ISSUE_TEMPLATE/*       # EXISTANT
│   ├── PULL_REQUEST_TEMPLATE.md  # EXISTANT
│   ├── dependabot.yml         # NOUVEAU
│   └── CODEOWNERS             # NOUVEAU
├── crates/
│   ├── claudine-core/         # cli.rs + commands/* + tui/* + logique  (gagne clap + ratatui + anyhow)
│   │   └── src/{lib.rs, cli.rs, commands/{mod,export,import,homes,tui}.rs, tui/*, <modules logique>}
│   └── claudine/              # binaire mince
│       ├── src/main.rs        # shim
│       └── tests/cli.rs       # intégration
├── packaging/
│   ├── aur/PKGBUILD           # NOUVEAU (depuis arch/ existant)
│   ├── homebrew/claudine.rb   # NOUVEAU
│   └── winget/                # EXISTANT (claudine est multi-OS)
├── site/                      # NOUVEAU — Zola
│   ├── config.toml
│   ├── content/_index.md
│   ├── templates/{base,index}.html
│   └── sass/main.scss
├── CONVENTIONS.md             # NOUVEAU
├── CLAUDE.md                  # NOUVEAU
├── AGENTS.md                  # NOUVEAU (pointeur vers CLAUDE/CONVENTIONS/CONTRIBUTING)
├── README.md                  # MODIFIÉ (badges, hero, install, lien site)
├── CHANGELOG.md               # MODIFIÉ (entrée [0.1.0])
├── CONTRIBUTING.md / CODE_OF_CONDUCT.md / SECURITY.md   # EXISTANT
└── LICENSE-MIT / LICENSE-APACHE   # EXISTANT
```

*(Pas de `MIGRATION.md` : c'est un artefact du template, pas d'un projet aligné.)*

---

## 5. Restructuration du code (le portage)

| Aujourd'hui | Cible |
|---|---|
| `claudine/src/main.rs` (`#[derive(Parser)] Cli`, dispatch export/import/homes, défaut→TUI) | `claudine-core/src/cli.rs` (`Cli` + dispatch mince) et `claudine-core/src/lib.rs::run()` |
| `claudine/src/cli.rs` (structs/impl des sous-commandes) | fondu dans `claudine-core/src/commands/*` |
| sous-commandes `export`, `import`, `homes add/remove` | `commands/{export,import,homes}.rs` (Args + `run()` + logique testée) |
| défaut (aucune sous-commande) → lance la TUI | `commands/tui.rs` (ou branche par défaut de `cli.rs`) appelle la TUI |
| `claudine/src/tui/{app,ui,mod,hooks_editor,mcp_editor,settings_form,marketplaces}.rs` | `claudine-core/src/tui/*` (déplacé tel quel) |
| modules logique `claudine-core/src/{home,settings,config,model,manifest,marketplaces,extensions,export,import,housekeeping,scan,search,remap,pathcodec,error}.rs` | **inchangés** (restent dans le core) |
| `claudine/src/main.rs` | `fn main() -> ExitCode { claudine_core::run() }` |

Déplacement des dépendances :
- `claudine-core/Cargo.toml` **gagne** : `clap` (derive), `ratatui` (crossterm via ratatui),
  `anyhow` — en plus de l'existant (`serde`, `serde_json` preserve_order, `thiserror`,
  `tar`, `flate2`).
- `claudine/Cargo.toml` ne garde que `claudine-core` en dépendance + les dev-deps de test
  (`assert_cmd`, `predicates`, `tempfile`).
- Toutes les versions passent en `.workspace = true` via `[workspace.dependencies]`, et
  chaque crate active `[lints] workspace = true`.

Contrainte : **aucune régression fonctionnelle**. Les 162 tests migrent avec leur code
(les tests TUI suivent `tui/*` dans le core ; les tests d'intégration restent dans le
binaire). Le comportement CLI/TUI est identique après portage.

---

## 6. Fondation & standards

1. **`Cargo.toml` (workspace)** : `edition = "2024"`, `rust-version = "1.85"`,
   `[workspace.lints.rust] unsafe_code = "forbid"`, `[workspace.lints.clippy] all = { level = "warn", priority = -1 }`,
   `[profile.release] lto = true, codegen-units = 1, strip = true`,
   `[workspace.dependencies]` consolidées.
2. **Migration edition 2024** : `cargo fix --edition` puis revue manuelle des changements
   (fermetures, `unsafe` attributs, etc.).
3. **Reformatage unique** : `cargo fmt` sur tout le dépôt, dans un **commit isolé**
   (diff volumineux mais purement mécanique), après quoi `fmt --check` passe.
4. **Fichiers créés** : `rust-toolchain.toml`, `rustfmt.toml` (`edition 2024`,
   `max_width = 100`), `deny.toml` (advisories + allow-list SPDX + bans + sources
   crates.io), `tarpaulin.toml` (workspace, exclusions IO/tui/main, cible 80 %),
   `.cargo/audit.toml` (miroir des ignores de `deny.toml`).
5. **Docs standards** : `CONVENTIONS.md` (source de vérité : edition/MSRV, fmt/lints,
   forme workspace, licences, Conventional Commits + Keep a Changelog + SemVer, politique
   langue EN docs / FR strings, porte qualité pré-PR), `CLAUDE.md` (guide agent : ordre de
   lecture, règles produit, table « où changer quoi », commandes de la porte qualité),
   `AGENTS.md` (pointeur court).

Vérifier `unsafe_code = "forbid"` : claudine n'utilise a priori aucun `unsafe`.

---

## 7. CI / release / packaging (calqués sur josephine)

### `ci.yml` (push + PR sur main)
- **lint** : `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings`.
- **test** : matrice ubuntu 22.04/24.04 + fedora 40/41 (containers) + **Windows + macOS**
  (claudine est multi-OS), `cargo test --workspace --locked`.
- **coverage** : `cargo tarpaulin` → Codecov (informationnel, non bloquant).
- **security** : `cargo audit` + `cargo deny check`.
- **bench-smoke** : `cargo bench --no-run --locked` (si des benches existent ; sinon étape
  no-op documentée — les benches criterion sont **hors périmètre** de ce chantier).

### `release.yml` (sur tag `v*.*.*`)
- **Conserve** le build multi-OS existant + les artefacts deb/rpm.
- **Ajoute** : rendu de la formule **Homebrew** (`.rb`, url + sha256 du tarball) et du
  **PKGBUILD AUR** (pkgver + sha256), attachés à la release ; publication **crates.io**
  *opt-in* via variable de dépôt `PUBLISH_CRATES == 'true'` (publie `claudine-core` puis
  `claudine`) ; notes de release auto-générées.
- Garde `winget/` (multi-OS).

### `packaging/`
- `aur/PKGBUILD` (dérivé de `packaging/arch/PKGBUILD` existant ; placeholder sha256 rempli
  au tag).
- `homebrew/claudine.rb` (formule ; `depends_on "rust" => :build`, install via cargo,
  sha256 rempli au tag, test `--version`).
- `winget/` conservé.

### `.github/`
- `dependabot.yml` (cargo + github-actions, hebdo).
- `CODEOWNERS`.

---

## 8. Partie web — `site/` Zola → GitHub Pages

Site statique mono-page, thème inspiré de josephine mais **identité propre à claudine**
(glyphe orange `#d97757`, pas de starfield).

- **`site/config.toml`** : `base_url = "https://systm-d.github.io/claudine"`, `title`,
  `description`, `compile_sass = true`, `[extra] brand_color = "#d97757"`,
  `repo_url = "https://github.com/systm-d/claudine"`.
- **`site/content/_index.md`** : front-matter `[extra]` (eyebrow, tagline, lede, CTA
  « View on GitHub » / « Install »), corps HTML en `<section>` :
  - **hero** : nom + glyphe Claude, tagline.
  - **features** : gérer sessions/projets, mémoire, config, extensions (hooks/MCP/plugins),
    marketplaces — grille de cartes.
  - **`#install`** : commandes deb / rpm / arch / homebrew / `cargo install` (+ winget).
  - **démo** : capture/`termsvg` de la TUI (aperçu de l'en-tête avec glyphe).
- **`site/templates/{base,index}.html`** : `base.html` (skeleton, `<head>` theme-color,
  footer avec `config.extra.repo_url`, `{% block content %}`), `index.html` (hero + rendu
  `{{ section.content | safe }}`).
- **`site/sass/main.scss`** : palette sous `:root` avec accent `--brand: #d97757`, styles
  hero / features grid / install / footer + media queries (reduced-motion, mobile).
- **`pages.yml`** : job **build** (checkout, `configure-pages`, install `zola`,
  `cd site && zola build`, upload `site/public`) + job **deploy** (`actions/deploy-pages`,
  environnement `github-pages`).

> **Go-live séparé** : GitHub Pages sur un dépôt **privé** requiert un plan payant ou le
> passage du dépôt en public. Ce chantier **construit** le site + le workflow ;
> l'activation réelle de Pages (et le passage open-source) est une décision distincte hors
> périmètre technique.

---

## 9. Tests & portes de validation

La porte qualité (identique à josephine, exécutée localement et en CI) :

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace          # les 162 tests migrent et restent verts
cargo build --release           # profil release (lto/strip) compile
cd site && zola build           # le site se construit
```

Ajouts de tests : `crates/claudine/tests/cli.rs` couvre `--version` et les sous-commandes
principales (export/import/homes) via `assert_cmd`.

---

## 10. Exécution

- Branche **`claudine-template-alignment`** (créée depuis `main`, qui contient déjà le
  commit de release `v0.0.2`).
- **Une** spec (ce document) → **un** plan multi-tâches (`writing-plans`) → **un** run SDD
  (implémenteurs + reviewers par tâche, revue finale opus sur toute la branche).
- Ordre interne des tâches : **A** fondation & portage (structure, edition, fmt, standards)
  → **B** CI/release/packaging → **C** site web. B et C dépendent de A (fmt --check, noms
  de crates, commandes d'install).
- **Merge sur `main` uniquement sur feu vert explicite** de l'utilisateur.
- À la clôture : bump version **`0.1.0`**, entrée `CHANGELOG [0.1.0]`, mise à jour des
  mémoires obsolètes (`claudine-no-cargo-fmt`, « core sans UI »).

---

## 11. Risques & mitigations

| Risque | Mitigation |
|---|---|
| Diff `cargo fmt` massif noie la revue | Commit `fmt` **isolé** et dédié, revu séparément (mécanique). |
| Edition 2024 casse des idiomes 2021 | `cargo fix --edition` + revue ciblée ; MSRV 1.85 dispo (rustc 1.95 installé). |
| `unsafe_code = "forbid"` refuse la compilation | Vérifier absence d'`unsafe` (attendu nul) avant d'activer. |
| Déplacer `tui/*` dans le core casse des imports/tests | Portage mécanique + `cargo test` après chaque déplacement ; aucune régression tolérée. |
| Pages sur dépôt privé ne déploie pas | Construction seulement ; go-live (public/Pages) hors périmètre, documenté. |
| Divergence de contenu du site (copie josephine) | Contenu **réécrit** pour claudine (features/install propres), pas un copier-coller. |

---

## 12. Hors périmètre (YAGNI)

- Benches criterion réels (seul `bench-smoke` no-op prévu ; benches = chantier ultérieur).
- Passage effectif en open-source, activation de Pages, publication crates.io (go-live).
- Documentation bilingue EN+FR du site (mono-langue pour commencer).
- Axes template non pertinents pour claudine (`state=sqlite`, `privileges=helper`,
  `service=daemon`).
- Édition de la mémoire dans la TUI (feature reportée, à construire **après** l'alignement,
  dans la nouvelle structure).
