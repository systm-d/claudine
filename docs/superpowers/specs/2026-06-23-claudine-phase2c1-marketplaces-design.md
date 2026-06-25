# Claudine — Phase 2c-1 : marketplaces & socle de gestion des plugins

- **Date** : 2026-06-23
- **Statut** : validé (design), prêt pour planification
- **Périmètre** : premier sous-projet de la phase 2c (« plugins »). 2c-1 = **socle** : helper git, modèle des marketplaces + manifeste, et opérations **add / list / remove / update** de marketplaces, avec un gestionnaire TUI. L'installation/désinstallation de plugins est le sous-projet **2c-2** (hors périmètre ici).

## 1. Contexte & objectif

Claude Code installe les plugins depuis des **marketplaces** : des dépôts git dont la racine contient `.claude-plugin/marketplace.json` (un catalogue de plugins). Le home actif tient le registre des marketplaces connues dans `plugins/known_marketplaces.json`, et clone chaque marketplace sous `plugins/marketplaces/<name>/`.

Format réel observé sur disque (`~/.claude-perso/plugins/`) :

```jsonc
// known_marketplaces.json
{
  "claude-plugins-official": {
    "source": { "source": "github", "repo": "anthropics/claude-plugins-official" },
    "installLocation": "/home/.../plugins/marketplaces/claude-plugins-official",
    "lastUpdated": "2026-06-25T07:54:22.246Z"
  }
}
```

```jsonc
// marketplaces/<name>/.claude-plugin/marketplace.json
{
  "$schema": "...",
  "name": "claude-plugins-official",
  "description": "...",
  "owner": { "name": "Anthropic", "email": "support@anthropic.com" },
  "plugins": [ { "name": "...", "description": "...", /* ... */ }, /* ... */ ]
}
```

Point clé : **la clé du registre = le `name` du manifeste** (ici `claude-plugins-official`), pas le nom du dépôt. Le `name` n'est donc connu **qu'après** clonage + lecture du manifeste.

2c-1 rend ce registre **gérable depuis Claudine** : ajouter une marketplace (clone + validation + enregistrement), les lister, en retirer, et les mettre à jour (pull).

### Critères de succès
- Depuis Extensions, ajouter / lister / retirer / mettre à jour des marketplaces du home actif, en écrivant `known_marketplaces.json` au format ci-dessus (sauvegarde préalable) et en clonant sous `plugins/marketplaces/<name>/`.
- Clonage **non bloquant** (thread d'arrière-plan + indicateur ; le TUI reste réactif).
- Aucune nouvelle dépendance de build ; MSRV **1.74 inchangée** (clonage délégué au binaire `git`).
- Tests cœur + TUI verts (fixtures git **locales**, sans réseau), clippy 0 warning.

## 2. Hors périmètre (2c-1)
- Installation / désinstallation / activation de **plugins** (`installed_plugins.json`, `enabledPlugins`) → **2c-2**.
- Affichage du catalogue des plugins d'une marketplace (lecture du `plugins[]` du manifeste pour le navigateur d'install) → **2c-2** (le modèle est néanmoins préparé ici, cf. §4).
- Clonage authentifié (dépôts privés / credentials), sous-modules, LFS.
- Sources non git.

## 3. Décision technique : clonage délégué au `git` système

Tout client https **en-process** (git2 → OpenSSL ; gix → rustls/aws-lc) impose un **compilateur C** au build (et gix relèverait la MSRV à 1.85 + tirerait tokio/reqwest/hyper). Le projet est aujourd'hui 100 % Rust pur, `cargo install` sans toolchain C. Pour préserver cette propriété **et** la MSRV 1.74, le clonage est **délégué au binaire `git`** via `std::process::Command` :
- `git` est déjà un prérequis de fait de l'écosystème Claude Code (les marketplaces *sont* des dépôts git) ; présence vérifiée (2.54.0).
- Aucune nouvelle crate, aucune dépendance de build, pas d'arbre async.

## 4. Résolution des chemins (cœur)

Pour un `home: &ClaudeHome` :
- `plugins_dir(home)            = home.base().join("plugins")`
- `known_marketplaces_path(home)= plugins_dir/known_marketplaces.json`
- `marketplaces_dir(home)       = plugins_dir/marketplaces`
- répertoire d'une marketplace   `= marketplaces_dir/<name>`
- manifeste                       `= marketplaces_dir/<name>/.claude-plugin/marketplace.json`

Si `plugins/` n'existe pas encore dans le home actif (cas `~/.claude` sans plugins), il est créé à la volée à l'ajout.

## 5. Modèle de données (cœur — nouveau module `marketplaces.rs`)

```rust
/// Provenance d'une marketplace (clé `source` du registre).
pub enum MarketplaceSource {
    Github { repo: String },   // {"source":"github","repo":"owner/repo"}
    Git    { url: String },    // {"source":"git","url":"https://..."}
    Local  { path: PathBuf },  // {"source":"local","path":"/abs/..."}
}

/// Entrée du registre `known_marketplaces.json`.
pub struct Marketplace {
    pub name: String,                 // clé du registre = name du manifeste
    pub source: MarketplaceSource,
    pub install_location: PathBuf,    // marketplaces/<name>
    pub last_updated: String,         // ISO 8601 UTC ms (cf. §7)
}

/// Manifeste `.claude-plugin/marketplace.json` (lecture).
pub struct MarketplaceManifest {
    pub name: String,
    pub description: Option<String>,
    pub owner_name: Option<String>,
    pub plugins: Vec<PluginManifestEntry>,
}

/// Entrée plugin du manifeste (minimal pour 2c-1 ; étendu en 2c-2).
pub struct PluginManifestEntry {
    pub name: String,
    pub description: Option<String>,
}
```

**URL de clonage** dérivée de la source :
- `Github { repo }` → `https://github.com/<repo>.git`
- `Git { url }`     → `url` tel quel
- `Local { path }`  → chemin local (git clone un dépôt local par chemin — sans réseau)

**Sérialisation `source`** dans le registre : `github` → `{source:"github",repo}` (fidèle à Claude Code) ; `git` → `{source:"git",url}` ; `local` → `{source:"local",path}`. Note de fidélité : seul `github` est attesté sur disque ; `git`/`local` sont notre meilleure correspondance (la sauvegarde + le fait que `installLocation` suffit à la résolution limitent le risque).

## 6. Helper git (cœur, sous-module `marketplaces::git`)

Fonctions synchrones encapsulant `std::process::Command::new("git")` ; le threading est côté TUI (§9).

- `git_clone(url: &str, dest: &Path) -> Result<()>` → `git clone --depth 1 <url> <dest>`.
- `git_pull(dir: &Path) -> Result<()>` → `git -C <dir> pull --ff-only` (repli : `fetch` + `reset --hard @{u}` si besoin — décision d'implémentation, `--ff-only` par défaut).
- Erreurs : code de sortie non nul ⇒ `CoreError` portant `stderr` (tronqué). `git` introuvable ⇒ message explicite « git introuvable dans le PATH ».
- `--depth 1` : clone superficiel (suffisant pour lire le manifeste et installer ; pas d'historique). `@sha` n'est pas requis en 2c-1 (réservé à l'install de plugins épinglés, 2c-2).

## 7. API cœur (`marketplaces.rs`)

- `read_marketplaces(home) -> Result<Vec<Marketplace>>` — lit `known_marketplaces.json` (objet `name → entrée`). Fichier absent ⇒ `Vec` vide.
- `read_marketplace_manifest(home, name) -> Result<MarketplaceManifest>` — parse le manifeste de la marketplace clonée.
- `add_marketplace(home, source) -> Result<Marketplace>` :
  1. `git_clone(url, tmp)` dans `marketplaces/.tmp-add-<provisoire>/` (provisoire = basename du dépôt/URL/chemin) ;
  2. lit + **valide** le manifeste (`name` non vide, `plugins` est un tableau) ; invalide ⇒ supprime `tmp` et renvoie `Err` (aucune écriture du registre) ;
  3. cible = `marketplaces/<manifest.name>` ; si elle existe déjà ⇒ `Err` « marketplace déjà présente » (supprime `tmp`) ;
  4. renomme `tmp` → cible ; écrit l'entrée dans `known_marketplaces.json` (`installLocation` = cible absolue, `last_updated` = maintenant) ;
  5. renvoie la `Marketplace`.
- `remove_marketplace(home, name) -> Result<()>` — retire l'entrée du registre **et** supprime le répertoire `marketplaces/<name>` (chemin **confiné** sous `marketplaces_dir`, refus sinon). Registre vide ⇒ `known_marketplaces.json` devient `{}`.
- `update_marketplace(home, name) -> Result<()>` — `git_pull(marketplaces/<name>)` + met `last_updated` à maintenant.

**Écriture du registre** via `SettingsDoc` (backup `.bak-<nanos>` + temp+rename + `preserve_order`), exactement comme les écritures de la phase 2. `known_marketplaces.json` est intégralement géré par Claudine (objet de marketplaces), donc on réécrit l'objet entier — mais via `SettingsDoc` pour bénéficier du backup/atomicité.

**Horodatage** `iso8601_utc(SystemTime) -> String` — helper pur `std` (algorithme civil days→Y-M-D, format `YYYY-MM-DDThh:mm:ss.mmmZ`), aucune dépendance ajoutée. Testé séparément sur un instant connu.

## 8. TUI — gestionnaire de marketplaces (modal)

Nouveau `crates/claudine/src/tui/marketplaces.rs`, ouvert depuis **Extensions** par la touche **`g`** (libre ; `Enter`/`p`/`m`/`t`/`E` sont pris). État `MarketplacesManager` dans `app.rs`, rendu `render_marketplaces` dans `ui.rs`, routage clavier prioritaire dans `mod.rs` (motif des autres modales).

- **Liste** : une ligne par marketplace — `nom · source · maj (date)`. `↑/↓` naviguent.
- `a` **ajouter** : invite de saisie d'une source. Heuristique de parsing :
  - contient `://` ou finit par `.git` → `Git { url }` ;
  - correspond à `^[\w.-]+/[\w.-]+$` → `Github { repo }` ;
  - chemin existant sur disque → `Local { path }` ;
  - sinon → erreur « source non reconnue ».
- `d` **retirer** : confirmation (réutilise le motif `confirm_delete`), puis `remove_marketplace`.
- `u` **mettre à jour** : `update_marketplace` sur la sélection.
- `Esc` ferme.

`a` et `u` déclenchent une opération **réseau** → exécutées en arrière-plan (§9). `d` est local/synchrone (suppression disque). Pendant un job, les touches de mutation (`a`/`d`/`u`) sont ignorées (un seul job à la fois).

## 9. Concurrence (clonage/pull non bloquants)

- App gagne `mkt_job: Option<MktJob>` avec `MktJob { label: String, rx: Receiver<MktJobOutcome>, frame: u8 }` et `MktJobOutcome { result: std::result::Result<String, String> }` (Ok(message) / Err(message)).
- `a`/`u` : l'app capture des **données possédées** (base du home, source, name) et `std::thread::spawn` qui exécute l'opération cœur complète (`add_marketplace` / `update_marketplace`), puis envoie l'`MktJobOutcome` via un `std::sync::mpsc::channel`. Aucun état d'`App` n'est partagé avec le thread.
- **Boucle d'évènements** : quand `mkt_job` est `Some`, on passe de `event::read()` (bloquant) à `event::poll(Duration::from_millis(120))` :
  - timeout → avance le spinner (`frame`), `try_recv()` ; rien → continue ;
  - `Ok(outcome)` → recharge la liste (`read_marketplaces`), pose le message de statut, efface `mkt_job` ;
  - évènement clavier → traité normalement.
- Indicateur : libellé `⠋ clonage de <source>…` (frames braille) en pied du modal tant que le job tourne.

## 10. Sûreté & validation
- Écriture de `known_marketplaces.json` : **backup + atomique** (`SettingsDoc`).
- **Rollback** du clone si le manifeste est invalide (suppression du `tmp`, registre intact).
- Suppression **confinée** : `remove_marketplace` refuse tout chemin hors `marketplaces_dir` (garde-fou sur le nom : pas de `/`, pas de `..`).
- **Un seul job réseau à la fois** ; touches de mutation ignorées pendant.
- Confirmation avant retrait.
- Multi-home : opère sur le **home actif** (cyclé par `t`).

## 11. Tests

**Cœur** (fixtures git **locales**, zéro réseau — un dépôt source créé via `git init` + commit dans un `tempdir`, avec un `.claude-plugin/marketplace.json` valide) :
- `add_marketplace(home, Local{path})` → `known_marketplaces.json` contient l'entrée (clé = `name` du manifeste, `source` local, `installLocation` correct), et `marketplaces/<name>/` existe avec le manifeste.
- Manifeste **invalide** (sans `name`) → `Err`, aucun fichier `known_marketplaces.json` écrit, pas de répertoire résiduel.
- Ajout en **double** (même `name`) → `Err` « déjà présente », registre inchangé.
- `read_marketplaces` relit l'entrée écrite (round-trip).
- `remove_marketplace` retire l'entrée **et** le répertoire ; garde-fou : un `name` contenant `..`/`/` → `Err` sans suppression.
- `update_marketplace` (commit ajouté au dépôt source, puis pull) → `last_updated` mis à jour, contenu rafraîchi.
- `iso8601_utc` sur un instant connu → chaîne attendue.

**TUI** :
- Ouvrir le gestionnaire (`g` depuis Extensions) → liste affichée.
- Flux d'ajout avec une **source locale** (job synchrone simulé ou exécuté en ligne dans le test) → l'entrée apparaît dans la liste après complétion.
- Retrait avec confirmation → l'entrée disparaît.
- Parsing de source : `owner/repo` → Github ; `https://…​.git` → Git ; chemin existant → Local ; entrée farfelue → message d'erreur.

## 12. Suites
- **2c-2** — navigateur de plugins d'une marketplace (lecture `plugins[]`), **installation / désinstallation / activation** (`installed_plugins.json`, `enabledPlugins`, cache des fichiers de plugin), avec épinglage `@sha` au clone.
