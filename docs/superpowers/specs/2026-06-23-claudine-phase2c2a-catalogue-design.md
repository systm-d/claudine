# Claudine — Phase 2c-2a : navigateur de catalogue de plugins + désinstallation

- **Date** : 2026-06-23
- **Statut** : validé (design), prêt pour planification
- **Périmètre** : premier sous-projet de 2c-2 (« plugins : install/désinstall/activation »). 2c-2a = **lecture du catalogue + désinstallation + activer/désactiver inline**. L'**installation** (les 4 types de source, fetch git + cache) est le sous-projet **2c-2b** (hors périmètre ici).

## 1. Contexte & objectif

Phase 2c-1 a livré la gestion des **marketplaces** (registre, clone, add/remove/update) et le gestionnaire TUI (touche `g`). Une marketplace clonée contient un manifeste `.claude-plugin/marketplace.json` listant ses **plugins** (`plugins[]`). Par ailleurs, Claude Code tient l'état des plugins **installés** et **activés** du home :

- `<home>/plugins/installed_plugins.json` (`version: 2`) :
  ```jsonc
  { "version": 2,
    "plugins": {
      "<plugin>@<marketplace>": [
        { "scope": "user",
          "installPath": "<abs>/plugins/cache/<marketplace>/<plugin>/<version>",
          "version": "1.0.0", "installedAt": "<iso>", "lastUpdated": "<iso>" }
      ] } }
  ```
  La valeur d'une clé est un **tableau** d'installations (une par scope).
- `enabledPlugins` (dans `settings.json`) : `{ "<plugin>@<marketplace>": true }`.
- Fichiers du plugin matérialisés sous `plugins/cache/<marketplace>/<plugin>/<version>/`.

2c-2a permet, depuis le gestionnaire de marketplaces, de **parcourir le catalogue** d'une marketplace (ses `plugins[]`), de voir l'état **installé / activé**, de **désinstaller** et d'**activer/désactiver** (réutilise `set_plugin_enabled` de 2a).

### Critères de succès
- Depuis le gestionnaire de marketplaces, `Enter` sur une marketplace ouvre son catalogue ; chaque plugin est marqué installé/activé.
- Désinstaller un plugin (scope user) : supprime son dossier de cache **et** son entrée d'`installed_plugins.json` **et** sa clé d'`enabledPlugins`, avec sauvegarde.
- Activer/désactiver un plugin installé depuis le catalogue.
- Réutilise le parsing plugins existant (`extensions.rs`) — pas de duplication.
- Tests cœur + TUI verts (fixtures sur disque, sans réseau), clippy 0 warning.

## 2. Hors périmètre (2c-2a)
- **Installation** de plugins (clone des sources `url`/`git-subdir`/`relative-path`/`github`, écriture du cache + d'`installed_plugins.json`) → **2c-2b**.
- Portée **projet** des plugins.
- Plugins installés mais absents du manifeste de leur marketplace (non listés par le catalogue ; acceptable).

## 3. Cœur (`claudine-core/src/extensions.rs`)

L'essentiel du parsing existe déjà (`read_plugins` privé → `PluginEntry { name = "<plugin>@<marketplace>", enabled, version, scope }`, et `set_plugin_enabled`). On **réutilise** ce code :

- **Exposer** `pub fn read_installed_plugins(home: &ClaudeHome) -> Vec<PluginEntry>` — fin wrapper public sur le `read_plugins` existant (aucune logique de parsing dupliquée). `PluginEntry` est déjà exporté.
- **Nouveau** `pub fn uninstall_plugin(home: &ClaudeHome, plugin: &str, marketplace: &str) -> Result<()>` :
  1. clé = `format!("{plugin}@{marketplace}")` ;
  2. charge `installed_plugins.json` via `SettingsDoc` ; localise le tableau sous `["plugins", clé]` ; récupère l'`installPath` de l'entrée **scope `user`** ;
  3. supprime le dossier `installPath` — **confiné sous `plugins/cache/`** (sinon `Err`, aucune suppression) ;
  4. retire l'entrée user du tableau ; tableau vide ⇒ `unset(["plugins", clé])` ; sauvegarde (`SettingsDoc`, backup + atomique) ;
  5. charge `settings.json` via `SettingsDoc` ; `unset(["enabledPlugins", clé])` ; sauvegarde.
  - Clé absente d'`installed_plugins.json` ⇒ `Err(CoreError::Marketplace("plugin non installé : …"))` (réutilise la variante existante) ; on ne touche pas `enabledPlugins`.
- **Réutilise** `set_plugin_enabled(home, clé, bool)` (2a) pour activer/désactiver, et `read_marketplace_manifest` (2c-1) pour la liste du catalogue.

Re-export dans `lib.rs` : `read_installed_plugins, uninstall_plugin`.

## 4. Modèle TUI (`crates/claudine/src/tui/marketplaces.rs`)

Le gestionnaire gagne un **2ᵉ niveau** « catalogue » (motif des deux niveaux de l'éditeur MCP), porté par une sous-structure pour rester testable :

```rust
pub struct CatalogEntry {
    pub name: String,             // nom du plugin (manifeste)
    pub description: Option<String>,
    pub installed: bool,
    pub enabled: bool,
}

pub struct PluginCatalog {
    pub marketplace: String,
    pub entries: Vec<CatalogEntry>,
    pub idx: usize,
    pub confirm_uninstall: bool,
}
```

- `MarketplacesManager` gagne `pub catalog: Option<PluginCatalog>` ; `catalog.is_some()` ⇒ niveau catalogue.
- `PluginCatalog::new(marketplace: String, manifest_plugins: &[PluginManifestEntry], installed: &[PluginEntry]) -> Self` — **pur** (testable) : pour chaque plugin `p` du manifeste, clé = `"{p.name}@{marketplace}"`, `installed` = présence de la clé dans `installed`, `enabled` = `.enabled` de l'entrée correspondante.
- Navigation bornée (`move_sel`), `selected()` / `selected_name()`, `begin_uninstall()` (no-op si entrée non installée).

## 5. TUI — flux & raccourcis

- **Niveau liste** (existant) : `a` ajouter, `u` màj, `d` retirer une marketplace, `Enter` **ouvrir le catalogue**, `Esc` fermer.
- **Niveau catalogue** : liste `nom  [installé][activé]` (ou `(non installé)`) + description ; `↑/↓` naviguer ; `Espace` activer/désactiver (**uniquement si installé**) ; `d` désinstaller (**confirmation**, uniquement si installé) ; `Esc` retour à la liste.
- Câblage `App` (méthodes près des `mkt_*`) : `open_catalog` (Enter en liste : lit manifeste + `read_installed_plugins`, construit `PluginCatalog` ; manifeste illisible ⇒ statut d'erreur, reste en liste), `catalog_toggle_enable` (Espace : `set_plugin_enabled` + maj de l'entrée), `catalog_uninstall_confirmed` (`uninstall_plugin` + maj de l'entrée : `installed=false, enabled=false`), `catalog_close`.
- Routage : dans `handle_marketplaces_key`, si `catalog.is_some()` → touches catalogue (motif d'action différée pour `Espace`/`d`-confirm qui appellent `&mut app`). Les opérations sont **synchrones** (pas de réseau ⇒ pas de job d'arrière-plan).

## 6. Sûreté & validation
- Désinstallation : suppression **confinée** sous `plugins/cache/` (garde-fou : refuse un `installPath` hors de ce dossier) ; `installed_plugins.json` et `settings.json` écrits via `SettingsDoc` (backup `.bak-<nanos>` + atomique) ; **confirmation** avant désinstallation ; portée **user**.
- `Espace`/`d` ignorés sur un plugin non installé.
- Home actif.

## 7. Tests

**Cœur** (fixtures sur disque, sans réseau) :
- `read_installed_plugins` : parse la map + tableau d'installations (clé `<plugin>@<mkt>`, enabled depuis `enabledPlugins`).
- `uninstall_plugin` : dossier de cache supprimé + entrée retirée d'`installed_plugins.json` + clé retirée d'`enabledPlugins` ; les autres clés des deux fichiers préservées.
- Garde-fou : `installPath` hors `plugins/cache/` ⇒ `Err`, aucune suppression ni écriture.
- Clé inexistante ⇒ `Err` propre, `enabledPlugins` intact.

**TUI** :
- `PluginCatalog::new` : marquage installed/enabled correct à partir d'un manifeste + d'une liste d'installés (dont un activé, un installé-non-activé, un non installé).
- `Enter` ouvre le catalogue ; `Espace` bascule l'activation d'un installé ; désinstallation avec confirmation (entrée repasse non installée) ; `Espace`/`d` no-op sur non installé ; `Esc` revient à la liste.

## 8. Suite
- **2c-2b** — installation : matérialiser les 4 types de source (`url`, `git-subdir`, `relative-path`, `github`) dans `plugins/cache/<mkt>/<plugin>/<version>/`, écrire `installed_plugins.json` (+ activation par défaut), via le clone `git` épinglé `@sha` (réutilise le helper git de 2c-1).
