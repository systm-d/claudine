# Claudine — Phase 2c-2b : installation de plugins

- **Date** : 2026-06-23
- **Statut** : validé (design), prêt pour planification
- **Périmètre** : second et dernier sous-projet de 2c-2 (« plugins : install/désinstall/activation »). 2c-2b = **installation** d'un plugin depuis le catalogue d'une marketplace : matérialisation des fichiers dans le cache + écriture du registre + auto-activation. La lecture du catalogue, la désinstallation et l'activation inline ont été livrées en **2c-2a** (hors périmètre ici).

## 1. Contexte & objectif

Phase 2c-2a a livré le **navigateur de catalogue** (touche `g` → `Enter` sur une marketplace) qui marque chaque plugin `installé / activé`, plus la **désinstallation** et l'**activation/désactivation** inline. Il manque la dernière brique : **installer** un plugin non installé.

Inspection du disque réel (`~/.claude-perso/plugins`, manifeste officiel `claude-plugins-official`, 238 plugins) — modèle confirmé :

- Manifeste `marketplaces/<mkt>/.claude-plugin/marketplace.json`, entrées `plugins[]` ; chaque entrée a un champ **`source`** (4 formes, voir §3).
- À l'installation, Claude Code matérialise les fichiers sous `plugins/cache/<mkt>/<plugin>/<version>/`, où `<version>` provient du `plugin.json` du plugin (fallback `"unknown"` observé sur disque, ex. `code-review/unknown`, `skill-creator/unknown`). Plusieurs versions peuvent coexister (ex. `superpowers/5.1.0`, `6.0.0`, `6.0.2`, `6.0.3`).
- Le clone réseau passe par un dossier temporaire `plugins/cache/temp_git_<...>/` (motif observé sur disque) puis copie vers le dossier de version définitif.
- `plugins/installed_plugins.json` (`version: 2`) : `"<plugin>@<mkt>"` → **tableau** d'installations `{scope:"user", installPath:"<abs>/cache/<mkt>/<plugin>/<version>", version, installedAt, lastUpdated}`.
- `enabledPlugins` (dans `settings.json`) : `{ "<plugin>@<mkt>": true }`.

### Critères de succès
- Depuis le catalogue, `i` sur un plugin **non installé** l'installe : fichiers présents dans `cache/<mkt>/<plugin>/<version>/`, entrée ajoutée à `installed_plugins.json`, clé `enabledPlugins` à `true`, avec sauvegarde.
- Les 4 types de source du manifeste officiel sont gérés (`url`, `git-subdir`, `github`, relative-path).
- Les 3 types réseau (git) tournent en arrière-plan (réutilise le `MktJob` spinner de 2c-1) ; le relative-path est synchrone.
- Au succès, l'entrée du catalogue repasse `installed=true, enabled=true` sans rouvrir le catalogue.
- Réutilise le helper `git` privé + `iso8601_utc` de 2c-1, et `SettingsDoc` pour toutes les écritures.
- Tests cœur (fixtures locales, sans réseau) + TUI verts, clippy 0 warning.

## 2. Hors périmètre (2c-2b)
- **Mise à jour** de version d'un plugin déjà installé (= future commande `update` ; ici `i` ne cible que les non-installés).
- Portée **projet** des plugins (scope `user` uniquement).
- Résolution de dépendances inter-plugins.
- Vérification d'intégrité du `sha` *de contenu* (le second hash du type `github`) : on épingle le **commit git**, ce qui suffit à la reproductibilité.

## 3. Modèle des sources (vérifié sur le manifeste officiel)

| `source` | n | structure réelle | mécanisme |
|---|---|---|---|
| `url` | 122 | `{source:"url", url:"<repo>.git", sha, path?}` | git clone `url` + checkout `sha` ; sous-dossier `path` si présent (5 cas), sinon racine |
| `git-subdir` | 63 | `{source:"git-subdir", url, path, ref?, sha}` | git clone `url` + checkout `sha` + sous-dossier `path` (`ref` = tag, informatif) |
| `github` | 2 | `{source:"github", repo:"o/n", commit, sha}` | git clone `https://github.com/<repo>.git` + checkout `commit` |
| relative-path | 51 | chaîne `"./plugins/X"` | copie locale depuis `marketplaces/<mkt>/X/` (marketplace déjà clonée, **zéro réseau**) |

**Réduction à 2 mécanismes** : `url`/`git-subdir`/`github` = *git clone épinglé à un commit, sous-dossier optionnel* ; relative-path = *copie locale*. Le commit à extraire est `sha` pour `url`/`git-subdir`, `commit` pour `github`.

## 4. Cœur (`claudine-core/src/marketplaces.rs`)

`install_plugin` vit dans `marketplaces.rs` car il a déjà le `mod git` privé (`clone`/`pull` durcis) et `iso8601_utc` ; il écrit `installed_plugins.json`/`settings.json` via `SettingsDoc` (déjà utilisé par `uninstall_plugin` de 2c-2a).

### 4.1 Étendre `PluginManifestEntry`

Ajouter le champ `source` à l'entrée existante (qui ne portait que `name`/`description`) :

```rust
pub enum PluginSource {
    /// relative-path : sous-dossier de la marketplace clonée (pas de réseau).
    RelativePath { path: String },        // "./plugins/X"
    /// url / git-subdir / github : clone git épinglé à un commit.
    Git { url: String, commit: String, subdir: Option<String> },
}

pub struct PluginManifestEntry {
    pub name: String,
    pub description: Option<String>,
    pub source: PluginSource,
}
```

Le parsing de `read_marketplace_manifest` mappe les 4 formes JSON → ces 2 variantes :
- `"./..."` (string) → `RelativePath`.
- `{source:"url", url, sha, path?}` → `Git { url, commit: sha, subdir: path }`.
- `{source:"git-subdir", url, path, sha, ..}` → `Git { url, commit: sha, subdir: Some(path) }`.
- `{source:"github", repo, commit, ..}` → `Git { url: "https://github.com/{repo}.git", commit, subdir: None }`.
- Forme inconnue → l'entrée est **ignorée** (catalogue tolérant ; cohérent avec 2c-2a). *(Le rendu/clé du catalogue n'utilise toujours que `name`/`description` ; `source` ne sert qu'à l'install.)*

### 4.2 `install_plugin`

```rust
pub fn install_plugin(home: &ClaudeHome, marketplace: &str, plugin: &str) -> Result<()>
```

1. Charger le manifeste de `marketplace` ; localiser l'entrée `plugin` (sinon `Err(CoreError::Marketplace("plugin introuvable au catalogue : …"))`).
2. **Matérialiser** la source dans un dossier `src` :
   - `RelativePath{path}` : `src = marketplaces/<mkt>/<path>` (normalisé, **confiné** sous `marketplaces/<mkt>/` ; refuse `..`).
   - `Git{url, commit, subdir}` : `git::clone(url)` dans `cache/temp_git_<nanos>/` → `git::checkout(commit)` → `src = temp/<subdir>` (confiné sous le temp) ; temp **supprimé** à la fin (succès ou échec).
3. Lire `version` depuis `src/.claude-plugin/plugin.json` (ou `src/plugin.json`) ; absent ⇒ `"unknown"`.
4. `dest = cache/<mkt>/<plugin>/<version>/` ; copie récursive `src → dest` (**confinée** sous `cache/`). Si `dest` existe déjà ⇒ on le considère déjà matérialisé (idempotent) ou on le remplace proprement.
5. `installed_plugins.json` via `SettingsDoc` : sous `["plugins", "<plugin>@<mkt>"]`, ajouter/mettre à jour l'entrée scope `user` `{scope, installPath: dest (abs), version, installedAt, lastUpdated}` (ISO via `iso8601_utc`) ; sauvegarde.
6. `settings.json` via `SettingsDoc` : `set(["enabledPlugins", "<plugin>@<mkt>"], true)` ; sauvegarde.

`mod git` gagne un `checkout(repo_dir, commit)` (en plus de `clone`/`pull`). Clone réutilise le durcissement existant (rejet d'`url` commençant par `-`, `-c protocol.ext.allow=never`, `--`, `--depth 1` puis fetch du commit épinglé si nécessaire).

### 4.3 Backlog M1 (à traiter ici)

Dans `uninstall_plugin` (2c-2a), déplacer la suppression du dossier cache **après** les écritures registre, pour rendre le registre autoritaire et réduire la fenêtre d'entrée pendante si une écriture échoue.

Re-export dans `lib.rs` : `install_plugin`, `PluginSource` (et `PluginManifestEntry` déjà exporté).

## 5. Modèle & flux TUI

`PluginCatalog`/`CatalogEntry` (2c-2a) sont réutilisés tels quels. Niveau catalogue :

- **`i`** sur l'entrée sélectionnée **non installée** → lance l'install. (no-op si déjà installée.)
- Type **relative-path** ⇒ install **synchrone** (pas de réseau) ; au succès, l'entrée passe `installed=true, enabled=true`, statut « installé ».
- Type **git** ⇒ **job d'arrière-plan** : réutilise `MktJob { label, frame, rx }` + `MktOutcome` de 2c-1 ; spinner pendant le clone ; à la complétion (`tick_mkt_job`), on rafraîchit l'entrée du catalogue (`installed=true, enabled=true`) ou on affiche l'erreur. Le catalogue reste ouvert.
- `App` : méthode `catalog_install(...)` près des `mkt_*`/`catalog_*` ; pour le cas git, elle démarre le thread (comme `mkt_begin_add`/`mkt_begin_update`) en mémorisant *quelle* entrée rafraîchir au retour.
- Routage : dans `handle_marketplaces_key`, branche catalogue, ajouter `i` (motif d'action différée, comme `Espace`/`d`).
- Footer/aide du catalogue : ajouter `i installer`.

## 6. Sûreté & validation
- **Clone** : durcissement de 2c-1 conservé (rejet `-`, `protocol.ext.allow=never`, `--`, profondeur 1). `checkout` sur un commit épinglé.
- **Confinement** : source relative-path confinée sous `marketplaces/<mkt>/` ; sous-dossier git confiné sous le temp ; destination confinée sous `cache/` (refus de tout `..`, canonicalisation avant copie).
- **Temp** : `cache/temp_git_<nanos>/` supprimé en succès **et** en échec (pas de fuite — leçon de 2c-1).
- **Écritures** : `installed_plugins.json` et `settings.json` via `SettingsDoc` (backup `.bak-<nanos>` + écriture atomique temp+rename, préservation des autres clés).
- **Idempotence** : `i` ignoré sur un plugin déjà installé.
- Home actif requis.

## 7. Tests

**Cœur** (fixtures locales, **sans réseau** — on n'exerce que le chemin relative-path + le parsing/écritures ; le chemin git est couvert par le parsing de `PluginSource` et l'écriture registre, le clone réel n'est pas testé en CI) :
- Parsing : les 4 formes JSON de `source` → bonnes variantes `PluginSource` (dont `url` avec/sans `path`, `git-subdir`, `github` → URL construite, string relative-path) ; forme inconnue ignorée.
- `install_plugin` (source relative-path, fixture `marketplaces/<mkt>/plugins/X/` avec `plugin.json` version) : `cache/<mkt>/X/<version>/` créé avec les fichiers, entrée ajoutée à `installed_plugins.json` (installPath/version corrects), `enabledPlugins["X@<mkt>"] == true`, autres clés des deux fichiers préservées.
- `version` absent ⇒ dossier `…/unknown/` + `version:"unknown"`.
- Confinement : relative-path contenant `..` ⇒ `Err`, aucune écriture.
- Idempotence / plugin introuvable au catalogue ⇒ `Err` propre.
- M1 : `uninstall_plugin` — si l'écriture registre échoue (simulable), le cache n'est pas supprimé ; en succès, l'ordre registre-puis-cache est respecté (au moins : test que la suppression du cache n'a pas lieu avant les écritures — vérifiable via un cas d'erreur registre).

**TUI** :
- `i` sur une entrée non installée (chemin synchrone relative-path simulé via l'API cœur) ⇒ l'entrée repasse `installed=true, enabled=true`.
- `i` no-op sur une entrée déjà installée.
- (logique de job réutilisée de 2c-1 ; le rafraîchissement d'entrée au retour de job est testé via la fonction de mise à jour d'entrée, pure.)

## 8. Suite
- 2c-2b **clôt 2c** (marketplaces + catalogue + install/désinstall/activation). Reste éventuellement : mise à jour de version d'un plugin installé, portée projet.
- Chantier cosmétique indépendant noté : ajouter le **logo ASCII Claude Code** dans le TUI.
