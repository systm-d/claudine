# Changelog

Toutes les modifications notables de ce projet sont documentées ici.

Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/)
Versionnage : [Semantic Versioning](https://semver.org/lang/fr/)

---

## [Unreleased]

### Phase 2a — Édition des hooks et bascule des plugins (juin 2026)

Ajouté :
- Section Extensions dans la TUI : lecture et affichage des hooks, plugins et
  serveurs MCP.
- Éditeur de hooks : navigation par groupes, édition des champs et commandes,
  validation à l'enregistrement (évènement et commande non vides).
- Bascule des plugins (`p` depuis l'onglet Extensions) : toggle
  `enabledPlugins` dans `settings.json`.
- Logo Claude Code (créature pixel) dans l'en-tête.
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

[Unreleased]: https://github.com/systm-d/claudine/compare/HEAD...HEAD
