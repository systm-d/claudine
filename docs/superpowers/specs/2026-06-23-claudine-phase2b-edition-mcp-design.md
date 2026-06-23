# Claudine — Phase 2b : édition des serveurs MCP (portée utilisateur)

- **Date** : 2026-06-23
- **Statut** : validé (design), prêt pour planification
- **Périmètre** : sous-projet 2b de la phase 2 (« édition / write »). 2a (hooks + plugins) est mergé. La portée **projet** des serveurs MCP est hors périmètre (éventuel sous-projet ultérieur).

## 1. Contexte & objectif

La section **Extensions** affiche déjà les serveurs MCP en lecture (`McpEntry` : nom, portée, résumé). La phase 2a a rendu éditables les hooks et l'activation des plugins. Ce sous-projet rend éditables les **serveurs MCP de portée utilisateur** (clé racine `mcpServers`) du home actif.

Les serveurs MCP vivent dans un fichier `.claude.json` **propre au home**, volumineux et partagé avec Claude Code (il contient quantité d'autres clés : `projects`, compteurs, caches…). L'écriture doit donc préserver scrupuleusement tout le reste du fichier.

### Critères de succès
- Depuis Extensions, créer / modifier / supprimer des serveurs MCP de portée utilisateur, et enregistrer dans le `.claude.json` du home actif, avec sauvegarde préalable.
- Toutes les autres clés du `.claude.json` sont préservées à l'octet près de leur valeur (seule la clé `mcpServers` est réécrite).
- Validation bloquante des entrées invalides.
- Tests cœur + TUI verts, clippy 0 warning.

## 2. Hors périmètre (2b)
- Serveurs MCP de portée **projet** (`projects[<chemin>].mcpServers`).
- Champs MCP exotiques au-delà de ceux listés en §4 (préservés s'ils existaient ? voir note §5).
- Installation/découverte de serveurs.

## 3. Résolution du fichier cible

`mcp_config_path(home) -> PathBuf` choisit le fichier `.claude.json` à lire/écrire :
1. `<home.base>/.claude.json` s'il existe (cas `~/.claude-perso/.claude.json`) ;
2. sinon le fichier hérité voisin `<parent>/<nom>.json` s'il existe (cas `~/.claude.json` pour le home `~/.claude`) ;
3. sinon, par défaut, `<home.base>/.claude.json` (à créer).

C'est le même ordre de candidats que la lecture MCP existante (`mcp_config_candidates`), mais on retient **un seul** fichier (le premier existant, ou le défaut in-home).

## 4. Modèle de données (cœur)

```rust
pub enum McpTransport { Stdio, Http, Sse }

pub struct McpServer {
    pub name: String,
    pub transport: McpTransport,
    // stdio
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    // http / sse
    pub url: String,
    pub headers: Vec<(String, String)>,
}
```

Sérialisation JSON par transport :
- **stdio** : `{ "type": "stdio", "command": "...", "args": [...], "env": { ... } }` — `args`/`env` omis si vides.
- **http** / **sse** : `{ "type": "http"|"sse", "url": "...", "headers": { ... } }` — `headers` omis si vide.

Lecture : `type` absent ⇒ `stdio` par défaut (convention Claude Code).

## 5. API cœur (`claudine-core/src/extensions.rs`)

S'appuie sur `SettingsDoc` (générique sur un chemin de fichier JSON ; `load`/`set`/`unset`/`save` avec backup `.bak-<nanos>` + temp+rename + `preserve_order`).

- `mcp_config_path(home) -> PathBuf` (cf. §3).
- `read_user_mcp_servers(home) -> Vec<McpServer>` — lit la clé racine `mcpServers` du fichier résolu.
- `write_user_mcp_servers(home, servers: &[McpServer]) -> Result<()>` — charge le fichier résolu via `SettingsDoc`, **remplace uniquement la clé `mcpServers`** par l'objet reconstruit (les autres clés sont préservées), sauvegarde. Liste vide ⇒ `unset(["mcpServers"])`.

Note préservation : comme pour `write_hooks` en 2a, la reconstruction porte sur les champs connus (§4) ; d'éventuels champs inconnus d'un serveur individuel ne sont pas conservés (cas rare, accepté ; backup en place). Les **autres clés racine** du `.claude.json` (`projects`, etc.) sont, elles, intégralement préservées.

## 6. TUI — éditeur MCP dédié (modal)

Nouveau `crates/claudine/src/tui/mcp_editor.rs`, ouvert depuis Extensions par **`m`** (cohérent avec `Enter`=hooks, `p`=plugins). État `McpEditor` dans `app.rs`, rendu `render_mcp_editor` dans `ui.rs`, routage clavier prioritaire dans `mod.rs` (motif des autres modales). Navigation à deux niveaux :

- **Niveau « serveurs »** : liste `nom [transport]`. `↑/↓` naviguent, `a` ajoute (serveur stdio vide), `d` supprime (confirmation), `Enter` ouvre, `s` enregistre, `Esc` ferme.
- **Niveau « serveur »** : lignes de champs, selon le transport courant —
  - `nom` (texte) ;
  - `type` — bascule **stdio → http → sse** par `←/→` (sans entrer en saisie) ;
  - stdio : `command` (texte), `args` (sous-éditeur de liste), `env` (sous-éditeur clé/valeur) ;
  - http·sse : `url` (texte), `headers` (sous-éditeur clé/valeur).
  - `Enter` édite le champ sélectionné (saisie texte) ou entre dans le sous-éditeur (args/env/headers) ; `Esc` remonte.
- Les sous-éditeurs liste / clé-valeur réutilisent les motifs d'édition existants de `settings_form` (saisie `input_char`/`input_backspace`/`input_commit`/`input_cancel`, ajout/suppression d'éléments).

## 7. Raccourcis (section Extensions)
- `Enter` → éditeur de hooks (2a).
- `p` → bascule des plugins (2a).
- `m` → éditeur de serveurs MCP (2b).
- `t` → home cible (agrégé) ; `E` → édite `settings.json` dans `$EDITOR`.

## 8. Sûreté & validation
- Backup + écriture atomique + **préservation intégrale** du `.claude.json` (seule `mcpServers` est remplacée).
- Validation bloquante (statut + éditeur maintenu ouvert) : `nom` non vide ; si `stdio`, `command` non vide ; si `http`/`sse`, `url` non vide. Les paires `env`/`headers` à clé vide sont ignorées à l'écriture.
- Confirmation avant suppression d'un serveur.
- Multi-home : l'écriture cible le fichier résolu du **home actif** (cyclé par `t`).

## 9. Tests
**Cœur :**
- Round-trip `read_user_mcp_servers` → édition → `write_user_mcp_servers` → relecture : un serveur **stdio** (command + args + env) et un serveur **http** (url + headers).
- Préservation des autres clés du `.claude.json` (ex. `projects`, un compteur) après écriture.
- Résolution `mcp_config_path` : in-home prioritaire ; repli sur le fichier hérité voisin ; défaut in-home si aucun.
- Liste vide ⇒ la clé `mcpServers` est retirée.

**TUI :**
- Ouvrir l'éditeur, ajouter un serveur stdio + un arg + une paire env, `s` → le `.claude.json` contient le serveur attendu.
- Éditer puis supprimer (avec confirmation) un serveur ; basculer le transport.
- Validation : enregistrement bloqué si `command` vide pour un stdio (éditeur reste ouvert, statut explicite).

## 10. Suites (rappel)
- **2b-bis éventuel** — édition des serveurs MCP de portée **projet**.
- **2c** — installation / désinstallation de plugins depuis les marketplaces.
