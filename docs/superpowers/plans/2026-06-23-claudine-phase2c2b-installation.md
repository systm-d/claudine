# Claudine — Phase 2c-2b : installation de plugins — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Installer un plugin depuis le catalogue d'une marketplace (touche `i`) : matérialiser ses fichiers dans `plugins/cache/<mkt>/<plugin>/<version>/`, écrire l'entrée `installed_plugins.json` (scope user) et l'auto-activer.

**Architecture:** Le cœur (`claudine-core/src/marketplaces.rs`) gagne `install_plugin` qui réduit les 4 types de source du manifeste à 2 mécanismes — copie locale (relative-path) ou clone git épinglé à un commit (url/git-subdir/github) — réutilise le `mod git` privé, `iso8601_utc` et `SettingsDoc`. La TUI lance l'install en tâche de fond (mécanisme `MktJob`/spinner de 2c-1) et rafraîchit l'entrée du catalogue à la complétion.

**Tech Stack:** Rust (workspace 2 crates), serde_json (`preserve_order`), ratatui 0.28 (TUI), git système via `std::process::Command`.

## Global Constraints

- **NE JAMAIS lancer `cargo fmt`** — le dépôt est formaté à la main. Valider via `cargo clippy --workspace` (0 warning) et `cargo test --workspace`.
- **MSRV 1.74, edition 2021, zéro nouvelle dépendance.** `claudine-core` n'a aucune dépendance UI ; git délégué au binaire système.
- Toutes les écritures de fichiers JSON passent par `SettingsDoc` (backup `.bak-<nanos>` + écriture atomique temp+rename, préservation des autres clés).
- Durcissement git conservé : refus d'une URL/commit commençant par `-`, `-c protocol.ext.allow=never`, `GIT_TERMINAL_PROMPT=0`.
- Confinement strict : source relative-path sous `marketplaces/<mkt>/`, sous-dossier git sous le temp, destination sous `plugins/cache/`. Tout `..` est refusé.
- Tests sans réseau : le chemin git est exercé via un dépôt git **local** (clone depuis un chemin de fichier fonctionne hors ligne).
- Messages d'erreur/UI en français, cohérents avec l'existant.

---

## File Structure

- `crates/claudine-core/src/marketplaces.rs` — **modifié** : nouvel enum `PluginSource`, champ `source` sur `PluginManifestEntry`, parsing du `source` dans `parse_manifest`, helpers `git::clone_full`/`git::checkout`, `install_plugin` + helpers privés `copy_dir_recursive`/`read_plugin_version`.
- `crates/claudine-core/src/extensions.rs` — **modifié** : `uninstall_plugin` réordonné (backlog M1 : suppression du cache après les écritures registre).
- `crates/claudine-core/src/lib.rs` — **modifié** : re-export de `PluginSource` et `install_plugin`.
- `crates/claudine/src/tui/marketplaces.rs` — **modifié** : méthode pure `PluginCatalog::mark_installed` + mise à jour du helper de test `pm()` ; `CatalogEntry` inchangé.
- `crates/claudine/src/tui/app.rs` — **modifié** : enum `MktJobKind`, champ `kind` sur `MktJob`, `catalog_install`, `tick_mkt_job` branché sur `kind`.
- `crates/claudine/src/tui/mod.rs` — **modifié** : touche `i` (catalogue) + variante `Deferred::Install`.
- `crates/claudine/src/tui/ui.rs` — **modifié** : `render_plugin_catalog` affiche le spinner d'install + indice `i installer`.

---

## Task 1: Cœur — `PluginSource` + parsing du `source` du manifeste

**Files:**
- Modify: `crates/claudine-core/src/marketplaces.rs` (enum `PluginManifestEntry` ~143-147, `parse_manifest` ~206-235)
- Modify: `crates/claudine-core/src/lib.rs` (re-export ~38-42)
- Modify: `crates/claudine/src/tui/marketplaces.rs` (helper de test `pm` ~214-216)
- Test: dans `crates/claudine-core/src/marketplaces.rs` (`mod tests`)

**Interfaces:**
- Produces:
  - `pub enum PluginSource { RelativePath { path: String }, Git { url: String, commit: String, subdir: Option<String> } }` (derive `Debug, Clone, PartialEq, Eq`)
  - `PluginManifestEntry { pub name: String, pub description: Option<String>, pub source: Option<PluginSource> }`
  - `fn parse_plugin_source(v: &Value) -> Option<PluginSource>` (privé) — `v` est la **valeur du champ `source`** (chaîne `"./..."` ou objet `{source:"url"|"git-subdir"|"github", ...}`).

- [ ] **Step 1: Écrire les tests de parsing (échouent)**

Ajouter dans `mod tests` de `marketplaces.rs` :

```rust
#[test]
fn parse_manifest_extracts_plugin_sources() {
    let json = serde_json::json!({
        "name": "mkt",
        "plugins": [
            {"name":"rel","description":"d","source":"./plugins/rel"},
            {"name":"u","source":{"source":"url","url":"https://x/r.git","sha":"abc","path":"sub"}},
            {"name":"u2","source":{"source":"url","url":"https://x/r.git","sha":"def"}},
            {"name":"gs","source":{"source":"git-subdir","url":"https://x/g.git","path":"p","ref":"v1","sha":"123"}},
            {"name":"gh","source":{"source":"github","repo":"o/n","commit":"deadbeef","sha":"z"}},
            {"name":"weird","source":{"source":"mystery"}}
        ]
    });
    let m = super::parse_manifest(&json).unwrap();
    let by = |n: &str| m.plugins.iter().find(|p| p.name == n).unwrap();
    assert_eq!(
        by("rel").source,
        Some(PluginSource::RelativePath { path: "./plugins/rel".into() })
    );
    assert_eq!(
        by("u").source,
        Some(PluginSource::Git { url: "https://x/r.git".into(), commit: "abc".into(), subdir: Some("sub".into()) })
    );
    assert_eq!(
        by("u2").source,
        Some(PluginSource::Git { url: "https://x/r.git".into(), commit: "def".into(), subdir: None })
    );
    assert_eq!(
        by("gs").source,
        Some(PluginSource::Git { url: "https://x/g.git".into(), commit: "123".into(), subdir: Some("p".into()) })
    );
    assert_eq!(
        by("gh").source,
        Some(PluginSource::Git { url: "https://github.com/o/n.git".into(), commit: "deadbeef".into(), subdir: None })
    );
    // Source inconnue : entrée conservée (nom/description) mais source None.
    assert_eq!(by("weird").source, None);
    // Toutes les entrées restent listées (catalogue non régressé).
    assert_eq!(m.plugins.len(), 6);
}
```

- [ ] **Step 2: Lancer le test (échec attendu)**

Run: `cargo test -p claudine-core parse_manifest_extracts_plugin_sources`
Expected: FAIL — `parse_manifest` est privé/non visible OU le champ `source` n'existe pas (erreur de compilation).

- [ ] **Step 3: Ajouter l'enum, le champ et le parsing**

Dans `marketplaces.rs`, remplacer la définition de `PluginManifestEntry` :

```rust
/// Source d'installation d'un plugin (champ `source` du manifeste).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginSource {
    /// Chaîne `"./plugins/X"` : sous-dossier de la marketplace clonée (pas de réseau).
    RelativePath { path: String },
    /// `url` / `git-subdir` / `github` : clone git épinglé à un commit, sous-dossier optionnel.
    Git {
        url: String,
        commit: String,
        subdir: Option<String>,
    },
}

/// Entrée plugin du manifeste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifestEntry {
    pub name: String,
    pub description: Option<String>,
    /// `None` si la forme de `source` n'est pas reconnue (plugin non installable).
    pub source: Option<PluginSource>,
}
```

Ajouter le helper (au-dessus de `parse_manifest`) :

```rust
/// Analyse la valeur du champ `source` d'un plugin (chaîne relative ou objet typé).
fn parse_plugin_source(v: &Value) -> Option<PluginSource> {
    if let Some(s) = v.as_str() {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        return Some(PluginSource::RelativePath { path: s.to_string() });
    }
    let o = v.as_object()?;
    match o.get("source").and_then(|s| s.as_str())? {
        // `url` et `git-subdir` partagent la même mécanique : clone + checkout `sha`.
        "url" | "git-subdir" => Some(PluginSource::Git {
            url: o.get("url")?.as_str()?.to_string(),
            commit: o.get("sha")?.as_str()?.to_string(),
            subdir: o.get("path").and_then(|p| p.as_str()).map(String::from),
        }),
        "github" => Some(PluginSource::Git {
            url: format!("https://github.com/{}.git", o.get("repo")?.as_str()?),
            commit: o.get("commit")?.as_str()?.to_string(),
            subdir: None,
        }),
        _ => None,
    }
}
```

Dans `parse_manifest`, remplacer la construction de chaque `PluginManifestEntry` (le `.filter_map(...)`) par :

```rust
    let plugins = arr
        .iter()
        .filter_map(|p| {
            let po = p.as_object()?;
            Some(PluginManifestEntry {
                name: po.get("name")?.as_str()?.to_string(),
                description: po.get("description").and_then(|d| d.as_str()).map(String::from),
                source: po.get("source").and_then(parse_plugin_source),
            })
        })
        .collect();
```

- [ ] **Step 4: Re-exporter `PluginSource` et corriger le helper de test TUI**

Dans `lib.rs`, ajouter `PluginSource` à la ligne `pub use marketplaces::{...}` :

```rust
pub use marketplaces::{
    add_marketplace, iso8601_utc, read_marketplace_manifest, read_marketplaces,
    remove_marketplace, update_marketplace, Marketplace, MarketplaceManifest,
    MarketplaceSource, PluginManifestEntry, PluginSource,
};
```

Dans `crates/claudine/src/tui/marketplaces.rs`, mettre à jour le helper de test `pm` pour le nouveau champ :

```rust
    fn pm(name: &str, desc: Option<&str>) -> PluginManifestEntry {
        PluginManifestEntry { name: name.into(), description: desc.map(|s| s.to_string()), source: None }
    }
```

- [ ] **Step 5: Lancer les tests**

Run: `cargo test -p claudine-core parse_manifest_extracts_plugin_sources && cargo test --workspace`
Expected: PASS (tous). Les tests existants du catalogue (`crates/claudine/src/tui/marketplaces.rs`) compilent grâce au `source: None`.

- [ ] **Step 6: Clippy**

Run: `cargo clippy --workspace`
Expected: 0 warning.

- [ ] **Step 7: Commit**

```bash
git add crates/claudine-core/src/marketplaces.rs crates/claudine-core/src/lib.rs crates/claudine/src/tui/marketplaces.rs
git commit -m "feat(core): parse plugin source types from marketplace manifest"
```

---

## Task 2: Cœur — helpers git `clone_full` + `checkout`

**Files:**
- Modify: `crates/claudine-core/src/marketplaces.rs` (`mod git` ~281-326)
- Test: dans `crates/claudine-core/src/marketplaces.rs` (`mod tests`)

**Interfaces:**
- Consumes: `git::finish` (privé, existant).
- Produces (dans `mod git`) :
  - `pub fn clone_full(url: &str, dest: &Path) -> Result<()>` — clone historique complet (permet le checkout d'un commit quelconque).
  - `pub fn checkout(dir: &Path, commit: &str) -> Result<()>` — `git -C <dir> checkout --detach <commit>`.

- [ ] **Step 1: Écrire le test (échoue)**

Ajouter dans `mod tests` de `marketplaces.rs` (le helper `git(...)` et `make_repo` existent déjà) :

```rust
/// Renvoie le SHA HEAD d'un dépôt.
fn head_sha(repo: &StdPath) -> String {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .expect("git rev-parse");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn git_clone_full_then_checkout_pins_commit() {
    // Dépôt avec 2 commits : v1 puis v2 d'un même fichier.
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    git(&["init", "-q", "-b", "main"], root);
    git(&["config", "user.email", "t@t"], root);
    git(&["config", "user.name", "t"], root);
    std::fs::write(root.join("f.txt"), "v1").unwrap();
    git(&["add", "-A"], root);
    git(&["commit", "-q", "-m", "c1"], root);
    let sha1 = head_sha(root);
    std::fs::write(root.join("f.txt"), "v2").unwrap();
    git(&["add", "-A"], root);
    git(&["commit", "-q", "-m", "c2"], root);

    let dest = tempfile::tempdir().unwrap();
    let dest = dest.path().join("clone");
    super::git::clone_full(&root.to_string_lossy(), &dest).unwrap();
    super::git::checkout(&dest, &sha1).unwrap();
    assert_eq!(std::fs::read_to_string(dest.join("f.txt")).unwrap(), "v1");

    // Commit inexistant → Err.
    assert!(super::git::checkout(&dest, "0000000000000000000000000000000000000000").is_err());
    // URL/commit ressemblant à une option → Err (durcissement).
    assert!(super::git::clone_full("--upload-pack=evil", &dest).is_err());
    assert!(super::git::checkout(&dest, "-x").is_err());
}
```

- [ ] **Step 2: Lancer le test (échec attendu)**

Run: `cargo test -p claudine-core git_clone_full_then_checkout_pins_commit`
Expected: FAIL — `git::clone_full` / `git::checkout` n'existent pas (erreur de compilation).

- [ ] **Step 3: Ajouter les deux fonctions dans `mod git`**

Dans `marketplaces.rs`, à l'intérieur de `mod git { ... }` (après `pub fn clone`), ajouter :

```rust
    /// `git clone -- <url> <dest>` (historique **complet**, sans `--depth`) afin de
    /// pouvoir extraire ensuite n'importe quel commit épinglé. Durci comme `clone`.
    pub fn clone_full(url: &str, dest: &Path) -> Result<()> {
        if url.starts_with('-') {
            return Err(CoreError::Marketplace(format!("url invalide : {url}")));
        }
        let mut c = Command::new("git");
        c.arg("-c")
            .arg("protocol.ext.allow=never")
            .arg("clone")
            .arg("--")
            .arg(url)
            .arg(dest);
        c.env("GIT_TERMINAL_PROMPT", "0");
        finish(c, "git clone")
    }

    /// `git -C <dir> checkout --detach <commit>` : positionne l'arbre de travail
    /// sur le commit épinglé. Refuse un commit ressemblant à une option.
    pub fn checkout(dir: &Path, commit: &str) -> Result<()> {
        if commit.starts_with('-') {
            return Err(CoreError::Marketplace(format!("commit invalide : {commit}")));
        }
        let mut c = Command::new("git");
        c.arg("-C")
            .arg(dir)
            .arg("checkout")
            .arg("--detach")
            .arg(commit);
        c.env("GIT_TERMINAL_PROMPT", "0");
        finish(c, "git checkout")
    }
```

- [ ] **Step 4: Lancer le test**

Run: `cargo test -p claudine-core git_clone_full_then_checkout_pins_commit`
Expected: PASS.

- [ ] **Step 5: Clippy + commit**

```bash
cargo clippy --workspace
git add crates/claudine-core/src/marketplaces.rs
git commit -m "feat(core): add git clone_full + checkout helpers for pinned commits"
```

Expected clippy: 0 warning. (Si clippy signale `clone_full`/`checkout` comme jamais utilisés, c'est attendu jusqu'à la Task 4 ; ajouter temporairement `#[allow(dead_code)]` sur les deux fonctions et le retirer en Task 4. Le noter dans le message de commit le cas échéant.)

---

## Task 3: Cœur — `install_plugin` (relative-path) + écritures registre + auto-activation

**Files:**
- Modify: `crates/claudine-core/src/marketplaces.rs` (nouvelle fonction + helpers privés)
- Modify: `crates/claudine-core/src/lib.rs` (re-export `install_plugin`)
- Test: dans `crates/claudine-core/src/marketplaces.rs` (`mod tests`)

**Interfaces:**
- Consumes: `read_marketplace_manifest`, `iso8601_utc`, `is_safe_name`, `marketplaces_dir`, `SettingsDoc`, `crate::extensions::set_plugin_enabled`, `PluginSource`.
- Produces:
  - `pub fn install_plugin(home: &ClaudeHome, marketplace: &str, plugin: &str) -> Result<()>`
  - `fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()>` (privé)
  - `fn read_plugin_version(src: &Path) -> Option<String>` (privé)
- Note: la branche `PluginSource::Git` est un `todo` provisoire ici (`Err`), implémentée en Task 4. Les helpers `git::clone_full`/`checkout` ne sont pas encore appelés.

- [ ] **Step 1: Écrire les tests relative-path (échouent)**

Ajouter dans `mod tests` de `marketplaces.rs` :

```rust
/// Écrit une marketplace clonée fictive avec un manifeste et un plugin relative-path.
fn seed_rel_marketplace(home: &ClaudeHome, mkt: &str, plugin: &str, version: Option<&str>) {
    let mdir = home.plugins_dir().join("marketplaces").join(mkt);
    // Manifeste avec une entrée relative-path.
    let cp = mdir.join(".claude-plugin");
    std::fs::create_dir_all(&cp).unwrap();
    let manifest = format!(
        r#"{{"name":"{mkt}","plugins":[{{"name":"{plugin}","description":"d","source":"./plugins/{plugin}"}}]}}"#
    );
    std::fs::write(cp.join("marketplace.json"), manifest).unwrap();
    // Fichiers du plugin sous marketplaces/<mkt>/plugins/<plugin>/.
    let pdir = mdir.join("plugins").join(plugin);
    let pcp = pdir.join(".claude-plugin");
    std::fs::create_dir_all(&pcp).unwrap();
    let pj = match version {
        Some(v) => format!(r#"{{"name":"{plugin}","version":"{v}"}}"#),
        None => format!(r#"{{"name":"{plugin}"}}"#),
    };
    std::fs::write(pcp.join("plugin.json"), pj).unwrap();
    std::fs::write(pdir.join("SKILL.md"), "hello").unwrap();
}

#[test]
fn install_plugin_relative_path_materializes_and_enables() {
    let (_d, home) = home();
    seed_rel_marketplace(&home, "m", "p", Some("1.2.3"));

    install_plugin(&home, "m", "p").unwrap();

    // Fichiers copiés sous cache/<mkt>/<plugin>/<version>/.
    let dest = home.plugins_dir().join("cache/m/p/1.2.3");
    assert!(dest.join("SKILL.md").is_file(), "fichier copié");
    assert!(dest.join(".claude-plugin/plugin.json").is_file());

    // Entrée installed_plugins.json (scope user, installPath, version).
    let doc = SettingsDoc::load(&home.plugins_dir().join("installed_plugins.json")).unwrap();
    let arr = doc.get(&["plugins", "p@m"]).and_then(|v| v.as_array()).cloned().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].get("scope").and_then(|s| s.as_str()), Some("user"));
    assert_eq!(arr[0].get("version").and_then(|s| s.as_str()), Some("1.2.3"));
    assert_eq!(
        arr[0].get("installPath").and_then(|s| s.as_str()),
        Some(dest.to_string_lossy().as_ref())
    );

    // Auto-activé.
    let sdoc = SettingsDoc::load(&home.settings_file()).unwrap();
    assert_eq!(sdoc.get_bool(&["enabledPlugins", "p@m"]), Some(true));
}

#[test]
fn install_plugin_missing_version_uses_unknown() {
    let (_d, home) = home();
    seed_rel_marketplace(&home, "m", "p", None);
    install_plugin(&home, "m", "p").unwrap();
    assert!(home.plugins_dir().join("cache/m/p/unknown").join("SKILL.md").is_file());
}

#[test]
fn install_plugin_unknown_plugin_errors() {
    let (_d, home) = home();
    seed_rel_marketplace(&home, "m", "p", Some("1"));
    assert!(install_plugin(&home, "m", "absent").is_err());
    // Rien écrit.
    assert!(!home.plugins_dir().join("installed_plugins.json").exists());
}

#[test]
fn install_plugin_rejects_dotdot_in_relative_source() {
    let (_d, home) = home();
    let mdir = home.plugins_dir().join("marketplaces").join("m");
    let cp = mdir.join(".claude-plugin");
    std::fs::create_dir_all(&cp).unwrap();
    std::fs::write(
        cp.join("marketplace.json"),
        r#"{"name":"m","plugins":[{"name":"evil","source":"./../../etc"}]}"#,
    )
    .unwrap();
    assert!(install_plugin(&home, "m", "evil").is_err());
    assert!(!home.plugins_dir().join("cache").join("m").exists());
}
```

- [ ] **Step 2: Lancer les tests (échec attendu)**

Run: `cargo test -p claudine-core install_plugin_`
Expected: FAIL — `install_plugin` n'existe pas (erreur de compilation).

- [ ] **Step 3: Implémenter `install_plugin` + helpers**

Dans `marketplaces.rs`, ajouter les helpers privés (près des autres fonctions privées) :

```rust
/// Lit `version` depuis `<src>/.claude-plugin/plugin.json` (absent → None).
fn read_plugin_version(src: &Path) -> Option<String> {
    let p = src.join(".claude-plugin").join("plugin.json");
    let content = std::fs::read_to_string(&p).ok()?;
    let v: Value = serde_json::from_str(&content).ok()?;
    v.get("version").and_then(|x| x.as_str()).map(String::from)
}

/// Copie récursive de `src` vers `dest` (fichiers + dossiers ; liens symboliques ignorés).
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest).map_err(|e| CoreError::io(dest, e))?;
    for entry in std::fs::read_dir(src).map_err(|e| CoreError::io(src, e))? {
        let entry = entry.map_err(|e| CoreError::io(src, e))?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let ft = entry.file_type().map_err(|e| CoreError::io(&from, e))?;
        if ft.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ft.is_file() {
            std::fs::copy(&from, &to).map_err(|e| CoreError::io(&to, e))?;
        }
    }
    Ok(())
}
```

Puis ajouter `install_plugin` (après `update_marketplace`). La branche `Git` renvoie une erreur provisoire (Task 4) :

```rust
/// Installe un plugin (portée user) depuis le catalogue d'une marketplace :
/// matérialise ses fichiers dans `cache/<mkt>/<plugin>/<version>/`, écrit
/// `installed_plugins.json` et l'auto-active. Idempotent (réécrit la version).
pub fn install_plugin(home: &ClaudeHome, marketplace: &str, plugin: &str) -> Result<()> {
    if !is_safe_name(marketplace) {
        return Err(CoreError::Marketplace(format!(
            "nom de marketplace invalide : {marketplace}"
        )));
    }
    if !is_safe_name(plugin) {
        return Err(CoreError::Marketplace(format!("nom de plugin invalide : {plugin}")));
    }

    // 1. Localiser l'entrée du plugin et sa source.
    let manifest = read_marketplace_manifest(home, marketplace)?;
    let source = manifest
        .plugins
        .iter()
        .find(|p| p.name == plugin)
        .ok_or_else(|| {
            CoreError::Marketplace(format!("plugin introuvable au catalogue : {plugin}@{marketplace}"))
        })?
        .source
        .clone()
        .ok_or_else(|| {
            CoreError::Marketplace(format!("source de plugin non gérée : {plugin}@{marketplace}"))
        })?;

    let cache_root = home.plugins_dir().join("cache");

    // 2. Matérialiser la source dans `src` (`temp` = à nettoyer si clone).
    let (src, temp): (PathBuf, Option<PathBuf>) = match &source {
        PluginSource::RelativePath { path } => {
            let rel = path.trim_start_matches("./");
            if rel.is_empty() || rel.split('/').any(|c| c == ".." || c.is_empty()) {
                return Err(CoreError::Marketplace(format!("chemin de plugin invalide : {path}")));
            }
            let mkt_dir = marketplaces_dir(home).join(marketplace);
            let dir = mkt_dir.join(rel);
            if !dir.starts_with(&mkt_dir) || !dir.is_dir() {
                return Err(CoreError::Marketplace(format!(
                    "dossier de plugin introuvable : {}",
                    dir.display()
                )));
            }
            (dir, None)
        }
        PluginSource::Git { .. } => {
            // Implémenté en Task 4.
            return Err(CoreError::Marketplace(
                "installation git non encore implémentée".to_string(),
            ));
        }
    };

    // 3. Version (depuis plugin.json), sinon "unknown".
    let version = read_plugin_version(&src).unwrap_or_else(|| "unknown".to_string());

    // 4. Copier vers cache/<mkt>/<plugin>/<version>/ (confiné, idempotent).
    let copy_result = (|| -> Result<PathBuf> {
        let dest = cache_root.join(marketplace).join(plugin).join(&version);
        if !dest.starts_with(&cache_root) {
            return Err(CoreError::Marketplace("destination hors cache".to_string()));
        }
        if dest.exists() {
            std::fs::remove_dir_all(&dest).map_err(|e| CoreError::io(&dest, e))?;
        }
        copy_dir_recursive(&src, &dest)?;
        Ok(dest)
    })();
    if let Some(t) = &temp {
        let _ = std::fs::remove_dir_all(t);
    }
    let dest = copy_result?;

    // 5. Écrire l'entrée scope user d'installed_plugins.json.
    let key = format!("{plugin}@{marketplace}");
    let installed_path = home.plugins_dir().join("installed_plugins.json");
    let mut doc = SettingsDoc::load(&installed_path)?;
    if doc.get(&["version"]).is_none() {
        doc.set(&["version"], Value::Number(2u64.into()));
    }
    let now = iso8601_utc(SystemTime::now());
    let mut entry = Map::new();
    entry.insert("scope".into(), Value::String("user".into()));
    entry.insert("installPath".into(), Value::String(dest.to_string_lossy().into_owned()));
    entry.insert("version".into(), Value::String(version.clone()));
    entry.insert("installedAt".into(), Value::String(now.clone()));
    entry.insert("lastUpdated".into(), Value::String(now));
    let mut arr: Vec<Value> = doc
        .get(&["plugins", key.as_str()])
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    arr.retain(|x| x.get("scope").and_then(|s| s.as_str()) != Some("user"));
    arr.push(Value::Object(entry));
    doc.set(&["plugins", key.as_str()], Value::Array(arr));
    doc.save(&installed_path)?;

    // 6. Auto-activer (réutilise extensions.rs).
    crate::extensions::set_plugin_enabled(home, &key, true)
}
```

- [ ] **Step 4: Re-exporter `install_plugin`**

Dans `lib.rs`, ajouter `install_plugin` à la liste `pub use marketplaces::{...}` :

```rust
pub use marketplaces::{
    add_marketplace, install_plugin, iso8601_utc, read_marketplace_manifest, read_marketplaces,
    remove_marketplace, update_marketplace, Marketplace, MarketplaceManifest,
    MarketplaceSource, PluginManifestEntry, PluginSource,
};
```

- [ ] **Step 5: Lancer les tests**

Run: `cargo test -p claudine-core install_plugin_`
Expected: PASS (4 tests relative-path).

- [ ] **Step 6: Clippy + commit**

```bash
cargo clippy --workspace
git add crates/claudine-core/src/marketplaces.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): install_plugin for relative-path sources + registry write + auto-enable"
```

Expected clippy: 0 warning. (Retirer un éventuel `#[allow(dead_code)]` posé en Task 2 si `clone_full`/`checkout` ne sont toujours pas appelés — sinon le laisser jusqu'à Task 4.)

---

## Task 4: Cœur — `install_plugin` branche git (clone + checkout + sous-dossier + nettoyage temp)

**Files:**
- Modify: `crates/claudine-core/src/marketplaces.rs` (branche `PluginSource::Git` d'`install_plugin`)
- Test: dans `crates/claudine-core/src/marketplaces.rs` (`mod tests`)

**Interfaces:**
- Consumes: `git::clone_full`, `git::checkout` (Task 2), `copy_dir_recursive`, `read_plugin_version` (Task 3).

- [ ] **Step 1: Écrire les tests git (échouent)**

Ajouter dans `mod tests` de `marketplaces.rs` :

```rust
/// Dépôt git « source de plugin » avec plugin.json (version) + fichier, dans un
/// sous-dossier optionnel. Renvoie (tempdir, chemin, sha HEAD).
fn make_plugin_repo(subdir: Option<&str>, version: &str) -> (tempfile::TempDir, String, String) {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    git(&["init", "-q", "-b", "main"], root);
    git(&["config", "user.email", "t@t"], root);
    git(&["config", "user.name", "t"], root);
    let base = match subdir {
        Some(s) => root.join(s),
        None => root.to_path_buf(),
    };
    let cp = base.join(".claude-plugin");
    std::fs::create_dir_all(&cp).unwrap();
    std::fs::write(cp.join("plugin.json"), format!(r#"{{"name":"gp","version":"{version}"}}"#)).unwrap();
    std::fs::write(base.join("SKILL.md"), "git-body").unwrap();
    git(&["add", "-A"], root);
    git(&["commit", "-q", "-m", "init"], root);
    let sha = head_sha(root);
    (d, root.to_string_lossy().into_owned(), sha)
}

/// Écrit une marketplace fictive dont le plugin `gp` a une source git donnée.
fn seed_git_marketplace(home: &ClaudeHome, mkt: &str, source_json: &str) {
    let cp = home.plugins_dir().join("marketplaces").join(mkt).join(".claude-plugin");
    std::fs::create_dir_all(&cp).unwrap();
    let manifest = format!(r#"{{"name":"{mkt}","plugins":[{{"name":"gp","source":{source_json}}}]}}"#);
    std::fs::write(cp.join("marketplace.json"), manifest).unwrap();
}

#[test]
fn install_plugin_git_url_clones_and_pins() {
    let (_repo, url, sha) = make_plugin_repo(None, "3.0.0");
    let (_d, home) = home();
    seed_git_marketplace(&home, "m", &format!(r#"{{"source":"url","url":"{url}","sha":"{sha}"}}"#));

    install_plugin(&home, "m", "gp").unwrap();

    let dest = home.plugins_dir().join("cache/m/gp/3.0.0");
    assert_eq!(std::fs::read_to_string(dest.join("SKILL.md")).unwrap(), "git-body");
    let sdoc = SettingsDoc::load(&home.settings_file()).unwrap();
    assert_eq!(sdoc.get_bool(&["enabledPlugins", "gp@m"]), Some(true));
    // Aucun dossier temporaire résiduel.
    let temp_left = std::fs::read_dir(home.plugins_dir().join("cache"))
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().starts_with("temp_git_"));
    assert!(!temp_left, "temp nettoyé");
}

#[test]
fn install_plugin_git_subdir_uses_subdirectory() {
    let (_repo, url, sha) = make_plugin_repo(Some("plugins/gp"), "4.1.0");
    let (_d, home) = home();
    seed_git_marketplace(
        &home,
        "m",
        &format!(r#"{{"source":"git-subdir","url":"{url}","path":"plugins/gp","sha":"{sha}"}}"#),
    );

    install_plugin(&home, "m", "gp").unwrap();
    assert_eq!(
        std::fs::read_to_string(home.plugins_dir().join("cache/m/gp/4.1.0/SKILL.md")).unwrap(),
        "git-body"
    );
}

#[test]
fn install_plugin_git_bad_commit_cleans_temp_and_errors() {
    let (_repo, url, _sha) = make_plugin_repo(None, "1.0.0");
    let (_d, home) = home();
    seed_git_marketplace(
        &home,
        "m",
        &format!(r#"{{"source":"url","url":"{url}","sha":"0000000000000000000000000000000000000000"}}"#),
    );

    assert!(install_plugin(&home, "m", "gp").is_err());
    // Pas d'entrée registre, pas de temp résiduel.
    assert!(!home.plugins_dir().join("installed_plugins.json").exists());
    let cache = home.plugins_dir().join("cache");
    if cache.exists() {
        let temp_left = std::fs::read_dir(&cache).unwrap().flatten().count();
        assert_eq!(temp_left, 0, "ni temp ni cache résiduel");
    }
}
```

- [ ] **Step 2: Lancer les tests (échec attendu)**

Run: `cargo test -p claudine-core install_plugin_git`
Expected: FAIL — la branche `Git` renvoie « installation git non encore implémentée ».

- [ ] **Step 3: Implémenter la branche `PluginSource::Git`**

Dans `install_plugin`, remplacer le bras `PluginSource::Git { .. } => { ... return Err(...) }` par :

```rust
        PluginSource::Git { url, commit, subdir } => {
            std::fs::create_dir_all(&cache_root).map_err(|e| CoreError::io(&cache_root, e))?;
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let temp = cache_root.join(format!("temp_git_{ts}"));
            if temp.exists() {
                let _ = std::fs::remove_dir_all(&temp);
            }
            // Clone complet + checkout du commit épinglé ; nettoie le temp si échec.
            if let Err(e) = git::clone_full(url, &temp).and_then(|()| git::checkout(&temp, commit)) {
                let _ = std::fs::remove_dir_all(&temp);
                return Err(e);
            }
            // Sous-dossier optionnel, confiné sous le temp.
            let src = match subdir {
                Some(sd) => {
                    let sd = sd.trim_start_matches("./");
                    if sd.split('/').any(|c| c == "..") {
                        let _ = std::fs::remove_dir_all(&temp);
                        return Err(CoreError::Marketplace(format!("sous-dossier invalide : {sd}")));
                    }
                    temp.join(sd)
                }
                None => temp.clone(),
            };
            if !src.starts_with(&temp) || !src.is_dir() {
                let _ = std::fs::remove_dir_all(&temp);
                return Err(CoreError::Marketplace(
                    "sous-dossier de plugin introuvable".to_string(),
                ));
            }
            (src, Some(temp))
        }
```

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p claudine-core install_plugin`
Expected: PASS (relative-path + git).

- [ ] **Step 5: Clippy + tests complets + commit**

```bash
cargo clippy --workspace
cargo test --workspace
git add crates/claudine-core/src/marketplaces.rs
git commit -m "feat(core): install_plugin git branch (clone, checkout, subdir, temp cleanup)"
```

Expected: 0 warning, tous tests verts. (S'assurer que tout `#[allow(dead_code)]` provisoire de Task 2 est retiré.)

---

## Task 5: Cœur — backlog M1 : `uninstall_plugin` supprime le cache après les écritures registre

**Files:**
- Modify: `crates/claudine-core/src/extensions.rs` (`uninstall_plugin` ~515-586)
- Test: dans `crates/claudine-core/src/extensions.rs` (`mod tests`)

**Interfaces:**
- Inchangé : `pub fn uninstall_plugin(home, plugin, marketplace) -> Result<()>`. Seul l'ordre interne change (registre d'abord, suppression du cache en dernier).

- [ ] **Step 1: Écrire le test d'ordre (échoue)**

Ajouter dans `mod tests` d'`extensions.rs` :

```rust
#[cfg(unix)]
#[test]
fn uninstall_plugin_keeps_cache_when_registry_write_fails() {
    use std::os::unix::fs::PermissionsExt;
    let (_d, home) = home_with(&[("settings.json", r#"{"enabledPlugins":{"foo@m":true}}"#)]);
    let base = home.base.clone();
    let foo_cache = base.join("plugins/cache/m/foo/1.0.0");
    std::fs::create_dir_all(&foo_cache).unwrap();
    let installed = format!(
        r#"{{"version":2,"plugins":{{"foo@m":[{{"scope":"user","installPath":"{}","version":"1.0.0"}}]}}}}"#,
        foo_cache.display()
    );
    let plugins_dir = base.join("plugins");
    std::fs::write(plugins_dir.join("installed_plugins.json"), installed).unwrap();

    // Rend plugins/ non inscriptible → l'écriture d'installed_plugins.json échoue.
    let mut perms = std::fs::metadata(&plugins_dir).unwrap().permissions();
    perms.set_mode(0o555);
    std::fs::set_permissions(&plugins_dir, perms).unwrap();

    let res = uninstall_plugin(&home, "foo", "m");

    // Restaure les permissions avant toute assertion (teardown-on-panic).
    let mut perms = std::fs::metadata(&plugins_dir).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&plugins_dir, perms).unwrap();

    assert!(res.is_err(), "l'écriture registre doit échouer");
    assert!(foo_cache.exists(), "cache préservé car la suppression vient après le registre");
}
```

- [ ] **Step 2: Lancer le test (échec attendu)**

Run: `cargo test -p claudine-core uninstall_plugin_keeps_cache_when_registry_write_fails`
Expected: FAIL — actuellement le cache est supprimé **avant** l'écriture registre, donc `foo_cache` n'existe plus.

- [ ] **Step 3: Réordonner `uninstall_plugin`**

Dans `extensions.rs`, remplacer le corps de `uninstall_plugin` (après la récupération de `user_entry`) pour : valider le chemin de cache d'abord (sans supprimer), écrire le registre, puis supprimer le cache en dernier. Remplacer le bloc qui commence par `// Supprime le dossier de cache, confiné...` jusqu'au `Ok(())` final par :

```rust
    // Valide le chemin de cache à supprimer SANS le supprimer encore : le registre
    // doit être réécrit d'abord (autoritaire), la suppression du cache vient après.
    let to_delete: Option<PathBuf> = match user_entry.get("installPath").and_then(|p| p.as_str()) {
        Some(install_path) => {
            let path = PathBuf::from(install_path);
            let cache_root = home.plugins_dir().join("cache");
            let has_parent = path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir));
            if has_parent || !path.starts_with(&cache_root) || path == cache_root {
                return Err(CoreError::Marketplace(format!(
                    "chemin d'installation hors cache : {install_path}"
                )));
            }
            if path.exists() {
                let canon = std::fs::canonicalize(&path).map_err(|e| CoreError::io(&path, e))?;
                let canon_root =
                    std::fs::canonicalize(&cache_root).map_err(|e| CoreError::io(&cache_root, e))?;
                if !canon.starts_with(&canon_root) || canon == canon_root {
                    return Err(CoreError::Marketplace(format!(
                        "chemin d'installation hors cache : {install_path}"
                    )));
                }
                Some(canon)
            } else {
                None
            }
        }
        None => None,
    };

    // Écritures registre d'abord.
    let remaining: Vec<Value> = entries
        .into_iter()
        .filter(|e| e.get("scope").and_then(|s| s.as_str()) != Some("user"))
        .collect();
    if remaining.is_empty() {
        doc.unset(&["plugins", key.as_str()]);
    } else {
        doc.set(&["plugins", key.as_str()], Value::Array(remaining));
    }
    doc.save(&installed_path)?;

    // Retire la clé d'enabledPlugins (settings.json), si présente.
    let settings_path = home.settings_file();
    let mut sdoc = SettingsDoc::load(&settings_path)?;
    if sdoc.get(&["enabledPlugins", key.as_str()]).is_some() {
        sdoc.unset(&["enabledPlugins", key.as_str()]);
        sdoc.save(&settings_path)?;
    }

    // Supprime le dossier de cache en dernier (registre désormais cohérent).
    if let Some(canon) = to_delete {
        fs::remove_dir_all(&canon).map_err(|e| CoreError::io(&canon, e))?;
    }
    Ok(())
```

Note : `user_entry` est emprunté à `entries` ; calculer `to_delete` (qui lit `user_entry`) **avant** le `entries.into_iter()` qui consomme `entries`. L'ordre ci-dessus respecte cela. Si le borrow-checker proteste, extraire d'abord `let install_path = user_entry.get("installPath")...map(str::to_string);` puis libérer `user_entry` avant de consommer `entries`.

- [ ] **Step 4: Lancer les tests d'extensions**

Run: `cargo test -p claudine-core uninstall_plugin`
Expected: PASS — le nouveau test + les 4 tests existants (`removes_cache_entry_and_enabled`, `rejects_path_outside_cache`, `unknown_key_errors`, `rejects_dotdot_traversal`).

- [ ] **Step 5: Clippy + commit**

```bash
cargo clippy --workspace
git add crates/claudine-core/src/extensions.rs
git commit -m "fix(core): uninstall_plugin deletes cache after registry writes (M1)"
```

Expected clippy: 0 warning.

---

## Task 6: TUI — installation depuis le catalogue (touche `i`, job de fond, spinner)

**Files:**
- Modify: `crates/claudine/src/tui/marketplaces.rs` (`PluginCatalog`)
- Modify: `crates/claudine/src/tui/app.rs` (`MktJob`, `MktOutcome`, `tick_mkt_job`, nouvelle méthode)
- Modify: `crates/claudine/src/tui/mod.rs` (`handle_marketplaces_key`)
- Modify: `crates/claudine/src/tui/ui.rs` (`render_plugin_catalog`, appel + indice)
- Test: dans `crates/claudine/src/tui/marketplaces.rs` (`mod tests`)

**Interfaces:**
- Consumes: `claudine_core::install_plugin`, `read_marketplaces` (existant).
- Produces:
  - `PluginCatalog::mark_installed(&mut self, plugin: &str)` — met `installed=true, enabled=true` sur l'entrée nommée.
  - `pub enum MktJobKind { Marketplace, InstallPlugin { plugin: String } }` (derive `Debug, Clone`).
  - `MktJob { label, frame, rx, kind: MktJobKind }`.
  - `App::catalog_install(&mut self)`.

- [ ] **Step 1: Écrire le test de `mark_installed` (échoue)**

Ajouter dans `mod tests` de `crates/claudine/src/tui/marketplaces.rs` :

```rust
#[test]
fn catalog_mark_installed_sets_flags() {
    let manifest = vec![pm("a", None), pm("b", None)];
    let installed = vec![]; // rien d'installé au départ
    let mut cat = PluginCatalog::new("m".into(), &manifest, &installed);
    assert!(!cat.entries[0].installed);
    cat.mark_installed("a");
    assert!(cat.entries[0].installed && cat.entries[0].enabled);
    // L'autre entrée reste intacte ; un nom inconnu est ignoré.
    assert!(!cat.entries[1].installed);
    cat.mark_installed("absent");
}
```

- [ ] **Step 2: Lancer le test (échec attendu)**

Run: `cargo test -p claudine catalog_mark_installed_sets_flags`
Expected: FAIL — `mark_installed` n'existe pas.

- [ ] **Step 3: Ajouter `mark_installed`**

Dans `crates/claudine/src/tui/marketplaces.rs`, dans `impl PluginCatalog`, après `begin_uninstall` :

```rust
    /// Marque le plugin nommé comme installé + activé (après un job d'install réussi).
    pub fn mark_installed(&mut self, plugin: &str) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.name == plugin) {
            e.installed = true;
            e.enabled = true;
        }
    }
```

- [ ] **Step 4: Lancer le test**

Run: `cargo test -p claudine catalog_mark_installed_sets_flags`
Expected: PASS.

- [ ] **Step 5: Ajouter `MktJobKind` + champ `kind`, brancher `tick_mkt_job`, `catalog_install`**

Dans `crates/claudine/src/tui/app.rs` :

1. Ajouter l'import `install_plugin` à la liste `use claudine_core::{ ... }` (ordre alphabétique, près de `import_dry_run`/`list_trash`).

2. Après la struct `MktJob`, ajouter :

```rust
/// Nature d'un job de fond : opération marketplace (rafraîchit la liste) ou
/// installation de plugin (rafraîchit l'entrée du catalogue).
#[derive(Debug, Clone)]
pub enum MktJobKind {
    Marketplace,
    InstallPlugin { plugin: String },
}
```

3. Ajouter le champ `kind` à `MktJob` :

```rust
pub struct MktJob {
    /// Libellé affiché dans le spinner.
    pub label: String,
    pub frame: u8,
    pub rx: std::sync::mpsc::Receiver<MktOutcome>,
    pub kind: MktJobKind,
}
```

4. Dans `mkt_begin_add` et `mkt_begin_update`, à la construction `self.mkt_job = Some(MktJob { label, frame: 0, rx });`, ajouter `kind: MktJobKind::Marketplace,`.

5. Remplacer `tick_mkt_job` par :

```rust
    /// Avance le spinner et applique le résultat du job s'il est arrivé.
    pub fn tick_mkt_job(&mut self) {
        let Some(job) = self.mkt_job.as_mut() else {
            return;
        };
        job.frame = job.frame.wrapping_add(1);
        let outcome = match job.rx.try_recv() {
            Ok(o) => o,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => MktOutcome {
                result: Err("le job s'est interrompu".to_string()),
            },
        };
        let kind = job.kind.clone();
        self.mkt_job = None;
        match (&kind, &outcome.result) {
            // Installation réussie : l'entrée du catalogue passe installée + activée.
            (MktJobKind::InstallPlugin { plugin }, Ok(_)) => {
                if let Some(c) = self.marketplaces.as_mut().and_then(|m| m.catalog.as_mut()) {
                    c.mark_installed(plugin);
                }
            }
            // Opération marketplace : rafraîchir la liste (succès ou échec).
            (MktJobKind::Marketplace, _) => {
                let home = self.home().clone();
                let items = read_marketplaces(&home).unwrap_or_default();
                if let Some(m) = self.marketplaces.as_mut() {
                    m.set_items(items);
                }
            }
            _ => {}
        }
        self.status = Some(match outcome.result {
            Ok(msg) => msg,
            Err(e) => format!("Échec : {e}"),
        });
    }
```

6. Après `catalog_uninstall_confirmed`, ajouter :

```rust
    /// Installe le plugin sélectionné (non installé) en arrière-plan.
    pub fn catalog_install(&mut self) {
        if self.mkt_job.is_some() {
            return;
        }
        let info = self
            .marketplaces
            .as_ref()
            .and_then(|m| m.catalog.as_ref())
            .and_then(|c| {
                c.selected()
                    .filter(|e| !e.installed)
                    .map(|e| (c.marketplace.clone(), e.name.clone()))
            });
        let Some((mkt, plugin)) = info else {
            return;
        };
        let home = self.home().clone();
        let label = format!("installation de {plugin}");
        let (tx, rx) = std::sync::mpsc::channel();
        let mkt_c = mkt.clone();
        let plugin_c = plugin.clone();
        std::thread::spawn(move || {
            let result = install_plugin(&home, &mkt_c, &plugin_c)
                .map(|()| format!("plugin « {plugin_c} » installé"))
                .map_err(|e| e.to_string());
            let _ = tx.send(MktOutcome { result });
        });
        self.mkt_job = Some(MktJob {
            label,
            frame: 0,
            rx,
            kind: MktJobKind::InstallPlugin { plugin },
        });
    }
```

- [ ] **Step 6: Câbler la touche `i` dans `handle_marketplaces_key`**

Dans `crates/claudine/src/tui/mod.rs`, fonction `handle_marketplaces_key` :

1. Ajouter une variante à l'enum local `Deferred` : `Install,`.

2. Dans la branche catalogue **hors confirmation** (`} else {` après `if c.confirm_uninstall {`), modifier les bras pour gérer `i` et neutraliser Espace/`d`/`i` pendant un job (`busy` est déjà calculé en haut de la fonction) :

```rust
                deferred = match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        c.move_sel(-1);
                        None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        c.move_sel(1);
                        None
                    }
                    KeyCode::Char(' ') if !busy => Some(Deferred::ToggleEnable),
                    KeyCode::Char('i') if !busy => Some(Deferred::Install),
                    KeyCode::Char('d') if !busy => {
                        c.begin_uninstall();
                        None
                    }
                    KeyCode::Esc => Some(Deferred::CatalogClose),
                    _ => None,
                };
```

3. Dans le `match deferred { ... }` final, ajouter le bras :

```rust
        Some(Deferred::Install) => app.catalog_install(),
```

- [ ] **Step 7: Afficher le spinner d'install dans le catalogue + indice `i`**

Dans `crates/claudine/src/tui/ui.rs` :

1. Importer `MktJob` : à l'`use` qui amène `MktMode`/`PluginCatalog` (depuis `crate::tui::...`), ajouter `use crate::tui::app::MktJob;` (ou compléter l'import existant d'`app`).

2. Au point d'appel dans `render_marketplaces`, passer le job courant :

```rust
    if let Some(c) = &m.catalog {
        render_plugin_catalog(c, app.mkt_job.as_ref(), f, area);
        return;
    }
```

3. Modifier la signature et le corps de `render_plugin_catalog` : ajouter le paramètre `job: Option<&MktJob>`, enrichir l'indice et ajouter une ligne spinner quand un job tourne. Remplacer l'en-tête de fonction et le bloc `hint` :

```rust
fn render_plugin_catalog(c: &PluginCatalog, job: Option<&MktJob>, f: &mut Frame, area: Rect) {
    let popup = centered_rect(78, 72, area);
    f.render_widget(Clear, popup);

    let hint = if c.confirm_uninstall {
        " o/n confirmer "
    } else if job.is_some() {
        " (installation en cours…) · Esc retour "
    } else {
        " Espace activer/désact. · i installer · d désinstaller · Esc retour "
    };
```

Puis, juste avant `f.render_widget(Paragraph::new(Text::from(lines)), inner);`, ajouter le spinner :

```rust
    if let Some(job) = job {
        const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let s = SPINNER[(job.frame as usize) % SPINNER.len()];
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {s} {}…", job.label),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )));
    }
```

4. Mettre à jour la ligne d'aide Extensions (~914) pour mentionner l'install :

```rust
        ("Extensions", "hooks (Enter) · plugins (p) · MCP (m) · marketplaces (g → Enter: catalogue, i installe) ; E édite settings.json"),
```

- [ ] **Step 8: Compiler, tester, clippy**

Run: `cargo build -p claudine && cargo test --workspace && cargo clippy --workspace`
Expected: build OK, tous tests verts, 0 warning.

- [ ] **Step 9: Commit**

```bash
git add crates/claudine/src/tui/marketplaces.rs crates/claudine/src/tui/app.rs crates/claudine/src/tui/mod.rs crates/claudine/src/tui/ui.rs
git commit -m "feat(tui): install plugins from catalog (i), background job + spinner"
```

---

## Self-Review

**1. Spec coverage**
- 4 types de source gérés → Task 1 (parsing `PluginSource`), Task 3 (relative-path), Task 4 (url/git-subdir/github via `Git`). ✓
- Matérialisation `cache/<mkt>/<plugin>/<version>/` + version depuis plugin.json (fallback `unknown`) → Task 3 (`read_plugin_version`, `copy_dir_recursive`). ✓
- Écriture `installed_plugins.json` (scope user) + auto-activation → Task 3 (étapes 5-6). ✓
- Réseau en arrière-plan (spinner) + relative-path → Task 6 (job de fond **uniforme** pour les deux ; déviation assumée vs « relative-path synchrone » de la spec : un seul chemin de code, le relative-path se termine en un tick). ✓
- Déclencheur `i`, refresh entrée → Task 6. ✓
- Confinement (marketplace/temp/cache), nettoyage temp, durcissement git → Tasks 2-4. ✓
- `SettingsDoc` partout → Tasks 3, 5. ✓
- Backlog M1 → Task 5. ✓
- Tests cœur sans réseau (relative-path + git via dépôt local) + TUI → Tasks 1-6. ✓

**2. Placeholder scan** — Aucun `TODO`/« add error handling » : la branche `Git` provisoire de Task 3 est un `Err` explicite, remplacé en Task 4 (transition testée). ✓

**3. Type consistency**
- `PluginManifestEntry.source: Option<PluginSource>` — cohérent Task 1 (def), Task 3/4 (`.source.clone()`), helper de test `pm` (`source: None`). ✓
- `install_plugin(home, marketplace, plugin)` — signature identique Tasks 3, 4, 6. ✓
- `MktJobKind` / `MktJob.kind` — défini et consommé en Task 6 (constructeurs add/update mis à jour). ✓
- `git::clone_full` / `git::checkout` — produits Task 2, consommés Task 4. ✓
- `PluginCatalog::mark_installed` — produit + consommé Task 6. ✓

## Execution Handoff

Plan complet et sauvegardé dans `docs/superpowers/plans/2026-06-23-claudine-phase2c2b-installation.md`.
