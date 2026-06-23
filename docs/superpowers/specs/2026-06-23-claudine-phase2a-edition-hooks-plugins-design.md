# Claudine — Phase 2a : édition des hooks + bascule des plugins

- **Date** : 2026-06-23
- **Statut** : validé (design), prêt pour planification
- **Périmètre** : sous-projet 2a de la phase 2 (« édition / write »). 2b (MCP) et 2c (installation de plugins) sont des sous-projets ultérieurs distincts.

## 1. Contexte & objectif

La section **Extensions** du TUI affiche aujourd'hui, en lecture seule, les hooks, plugins et serveurs MCP du home actif (`crates/claudine-core/src/extensions.rs` + `render_extensions` dans `ui.rs`). La phase 2 vise à rendre ces réglages **éditables depuis le TUI**.

Ce sous-projet (2a) couvre les deux cibles qui vivent dans `settings.json` du home, donc les plus sûres et qui réutilisent l'infrastructure d'écriture existante :

1. **Hooks** — éditeur dédié plein écran (création / édition / suppression).
2. **Plugins** — bascule activer / désactiver (`enabledPlugins`).

### Critères de succès
- Depuis Extensions, l'utilisateur peut créer, modifier et supprimer des hooks, et enregistrer dans `settings.json` du home actif, avec sauvegarde automatique préalable.
- L'utilisateur peut activer / désactiver un plugin installé et enregistrer.
- Aucune perte de données : backup avant écriture, écriture atomique, préservation des clés inconnues.
- Tests cœur + TUI verts, clippy propre.

## 2. Hors périmètre (2a)
- Édition des serveurs MCP (→ 2b).
- Installation / désinstallation de plugins, marketplaces, réseau (→ 2c).
- Édition des hooks via `$EDITOR` (déjà possible avec `E` sur `settings.json`, conservé tel quel).

## 3. Modèle de données (cœur)

`extensions.rs` conserve son volet lecture (`HookEntry`, `PluginEntry`, `McpEntry`, `read_extensions`) et gagne un modèle d'édition plus riche pour les hooks (niveau « Complet » : `type` + `timeout`) :

```rust
pub struct HookCommand {
    pub kind: String,            // "command" (valeur par défaut)
    pub command: String,
    pub timeout: Option<u64>,    // secondes, optionnel
}

pub struct HookGroup {
    pub event: String,           // ex. "PreToolUse"
    pub matcher: Option<String>, // surtout pour PreToolUse / PostToolUse
    pub commands: Vec<HookCommand>,
}
```

Structure JSON cible dans `settings.json` :
```json
"hooks": {
  "<Event>": [
    { "matcher": "<opt>", "hooks": [ { "type": "command", "command": "...", "timeout": 30 } ] }
  ]
}
```

## 4. API cœur (`claudine-core`)

S'appuie sur `SettingsDoc` (chargement, `get`/`set`/`unset` imbriqués, sauvegarde avec backup `.bak-<nanos>` + temp+rename + `preserve_order`).

- `read_hook_groups(home: &ClaudeHome) -> Vec<HookGroup>` — lecture structurée (niveau édition) depuis `settings.json` (+ `settings.local.json` ignoré ici : on n'édite que `settings.json`).
- `write_hooks(home: &ClaudeHome, groups: &[HookGroup]) -> Result<()>` — réécrit l'objet `hooks` **en mutant l'arbre JSON existant** plutôt qu'en le reconstruisant à plat, afin de préserver d'éventuels champs inconnus d'une commande et l'ordre des clés. Sauvegarde via `SettingsDoc`.
- `set_plugin_enabled(home: &ClaudeHome, name: &str, enabled: bool) -> Result<()>` — pose `enabledPlugins.<name> = bool` et sauvegarde.

Note préservation : pour les champs d'une commande au-delà de `type`/`command`/`timeout` (rares), l'écriture conserve les champs présents sur l'entrée d'origine quand elle est éditée en place ; une commande nouvellement créée ne contient que `type`/`command`/`timeout`.

## 5. TUI — éditeur de hooks (modal dédié)

Ouvert depuis la section Extensions par **`Enter`**. État applicatif dédié (`HooksEditor`) dans `app.rs`, rendu par `render_hooks_editor` dans `ui.rs`, routage clavier prioritaire dans `mod.rs` (comme les autres modales).

Navigation hiérarchique à deux niveaux :

- **Niveau « groupes »** : liste des `HookGroup` affichés `évènement [matcher]`.
  - `↑/↓` naviguent, `a` ajoute un groupe, `d` supprime (confirmation), `Enter` entre dans le groupe, `s` enregistre, `Esc` ferme.
- **Niveau « groupe »** : édition des champs d'un groupe.
  - `évènement` : choisi dans une **liste curatée** des évènements Claude Code connus (PreToolUse, PostToolUse, PostToolUseFailure, UserPromptSubmit, Notification, Stop, SubagentStart, SubagentStop, SessionStart, SessionEnd, PreCompact, TaskCompleted, WorktreeCreate, WorktreeRemove…) **avec saisie libre en repli** (pour ne pas bloquer un nouvel évènement).
  - `matcher` : champ texte optionnel.
  - **commandes** : liste éditable (`a` ajoute, `Enter` édite, `d` supprime). Chaque commande édite `command` (texte) et `timeout` (nombre optionnel) ; `type` par défaut `"command"`.
  - `Esc` remonte au niveau groupes.

La saisie d'un champ scalaire réutilise le motif d'entrée texte existant (cf. `settings_form` : `input_char`/`input_backspace`/`input_commit`/`input_cancel`).

## 6. TUI — bascule des plugins (modal)

Dans Extensions, **`p`** ouvre un petit modal listant les plugins installés avec leur état (`✓` activé / `✗` désactivé) :
- `↑/↓` naviguent, **Espace** bascule l'état de la ligne, `s` enregistre (un `set_plugin_enabled` par plugin modifié), `Esc` ferme.
- Pas d'installation ni de suppression de plugin.

## 7. Raccourcis (section Extensions)
- `Enter` → éditeur de hooks.
- `p` → modal de bascule des plugins.
- `t` → change le home cible (déjà existant, en agrégé).
- `E` → édite `settings.json` dans `$EDITOR` (déjà existant, conservé).

## 8. Sûreté & validation
- **Backup** systématique avant écriture (réutilisé de `SettingsDoc`), **écriture atomique** temp+rename.
- **Préservation** des autres réglages et des clés inconnues (`preserve_order` + mutation en place).
- **Confirmation** avant suppression d'un groupe ou d'une commande (réutilise le motif de confirmation existant).
- **Validation** : `command` non vide ; `timeout` numérique ≥ 0 ou vide ; `évènement` non vide. Une entrée invalide bloque l'enregistrement avec un message de statut.
- **Multi-home** : toutes les écritures ciblent le home actif (`app.home()` / `active`, cyclé par `t`).

## 9. Tests
**Cœur :**
- Round-trip : `read_hook_groups` → modification → `write_hooks` → `read_hook_groups` redonne l'état attendu.
- Préservation des autres clés de `settings.json` après `write_hooks`.
- Préservation d'un champ inconnu sur une commande éditée.
- `set_plugin_enabled` pose/retire correctement et préserve les autres réglages.

**TUI :**
- Ouvrir l'éditeur, ajouter un groupe + une commande, `s` → `settings.json` contient le hook attendu.
- Éditer puis supprimer (avec confirmation) un groupe / une commande.
- Modal plugins : basculer un plugin, `s` → `enabledPlugins` reflète l'état.

## 10. Suites (rappel)
- **2b** — édition des serveurs MCP dans `<home>/.claude.json` (fichier global partagé) : écriture sûre, add/edit/remove.
- **2c** — installation / désinstallation de plugins depuis les marketplaces (réseau, intégrité, cache).
