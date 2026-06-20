# Claudine — TUI Rust pour Claude Code

- **Date :** 2026-06-20
- **Statut :** Design validé (en attente de relecture utilisateur)
- **Auteur :** k@levilainpetit.dev (avec Claude Code)

## 1. Contexte & objectif

Construire **Claudine**, un outil Rust pour naviguer et gérer les données locales de
Claude Code (`~/.claude`) : sessions, mémoire, configuration, plugins/skills/agents.

L'objectif à terme est un **TUI complet** (panneau de contrôle). Le besoin déclencheur,
concret et daté, est la **migration vers un nouveau PC** : pouvoir exporter, importer et
**déplacer/remapper** ses sessions sans rien perdre, sachant que les chemins de projets
seront **probablement différents** sur la nouvelle machine.

On construit donc par **phases livrables**, la phase 1 sécurisant la migration via un
cœur testé et une CLI, le TUI venant ensuite par-dessus le même cœur.

## 2. Décisions clés (cadrage)

| Sujet | Décision |
|---|---|
| Périmètre produit | « Tout » (explorer, éditer, ranger, rechercher, migrer) — **construit en phases** |
| Besoin prioritaire | Export / import / **remap** des sessions (migration PC) |
| Timing | Réinstallation à venir, mais **pas bloquante** : on peut finir Claudine d'abord |
| Contenu de la migration | Sessions + mémoire + config/MCP + plugins/skills/agents (**hors secrets**) |
| Chemins sur le nouveau PC | **Probablement différents** → moteur de remap nécessaire |
| Approche de construction | **B** : `claudine-core` (testé) + CLI d'abord, puis TUI sur le même cœur |
| Nom du binaire | `claudine` |
| Conflit à l'import | **Skip l'existant par défaut**, `--overwrite` en option |
| `history.jsonl` | **Inclus par défaut**, catégorie désactivable |

## 3. Architecture

Workspace Cargo, séparation nette cœur / interface :

```
claude-tui/                  (workspace, le repo)
├─ crates/
│  ├─ claudine-core/  (lib)  ← tout le savoir sur ~/.claude : modèle, scan,
│  │                            export, import + remap. AUCUNE dépendance UI. Testé en TDD.
│  └─ claudine/       (bin)  ← un seul binaire :
│                              • `claudine`           → lance le TUI (ratatui)
│                              • `claudine export …`  → CLI (sous-commandes)
│                              • `claudine import …`  → CLI avec remap
└─ docs/superpowers/specs/   ← specs
```

**Un seul binaire `claudine`** : sans argument il ouvre le TUI ; avec une sous-commande
il agit en CLI. Les deux interfaces appellent exactement le **même `claudine-core`** →
zéro logique dupliquée, la partie risquée (remap) testée une seule fois.

### Briques techniques

| Besoin | Crate |
|---|---|
| TUI | `ratatui` + `crossterm` |
| Sérialisation jsonl & config | `serde` + `serde_json` |
| CLI / sous-commandes | `clap` (derive) |
| Archive de bundle | `tar` + `flate2` (`.tar.gz`) |
| Erreurs typées (cœur) | `thiserror` |
| Erreurs applicatives (bin) | `anyhow` |
| Tests sur faux `~/.claude` | `tempfile` |
| Tests CLI | `assert_cmd` + `predicates` |

## 4. Modèle de données (`claudine-core`)

### Résolution des chemins
`ClaudeHome::discover()` localise la base (`$CLAUDE_CONFIG_DIR` sinon `~/.claude`) ainsi
que `~/.claude.json`, et expose des accès typés aux sous-ressources. **Toutes** les
lectures du cœur passent par là, ce qui rend le cœur testable en pointant sur un faux home.

### Entités

- **`Project { encoded_name, cwd, sessions }`**
  Le nom de dossier encodé (`/` → `-`) est **ambigu** : `…-backend-generic-rag` peut venir
  de `/backend/generic/rag` ou `/backend/generic-rag`. La **source de vérité du `cwd`** est
  donc le champ `cwd` **à l'intérieur** des `.jsonl`, pas le décodage du nom de dossier. Le
  nom de dossier sert uniquement de clé de stockage ; au remap on **ré-encode** le nouveau `cwd`.
- **`SessionMeta { id, path, first_ts, last_ts, message_count, summary, size }`**
  Calculé à bas coût (premières/dernières lignes + comptage), sans charger toute la
  transcription. Sert au listing.
- **`SessionTranscript`** — lignes complètes, chacune conservée en `serde_json::Value` pour
  **préserver les champs inconnus** (compatibilité ascendante).
- **`Memory`** — `~/.claude/CLAUDE.md` (user). Note : la mémoire *projet* (`<projet>/CLAUDE.md`)
  vit dans les dépôts, pas sous `~/.claude` ; elle revient avec le dépôt re-cloné. Le TUI
  pourra la lire via le `cwd` du projet.
- **`Config`** — `settings.json`, `settings.local.json`, et une **vue assainie** de
  `~/.claude.json` (serveurs MCP, liste de projets) excluant secrets / tokens OAuth.
- **`Plugins`** — `~/.claude/plugins/`, skills, agents.

## 5. Format de bundle (export)

Un `.tar.gz` contenant :

```
manifest.json          schema_version, created_at, source_hostname, source_home,
                       projects[ { encoded_name, cwd, session_ids } ],
                       included_categories, excluded[]
projects/<encoded>/*.jsonl
todos/…
memory/CLAUDE.md
config/{settings.json, settings.local.json, claude.json.sanitized}
plugins/… skills/… agents/…
history.jsonl          (si catégorie activée)
```

Le `manifest.json` est la clé du remap : il enregistre le `cwd` réel de chaque projet,
indépendamment du nom de dossier encodé.

### Exclusions (secrets / spécifique machine)
`.credentials.json`, tokens OAuth dans `~/.claude.json`, `security_warnings_state_*`,
`cache/`, `shell-snapshots/`, `session-env/`, `telemetry/`. L'export est **strictement en
lecture seule** sur la source.

## 6. Flux export

1. `ClaudeHome::discover()` → scan → construction du modèle.
2. Sélection des catégories (toutes par défaut ; `history` désactivable).
3. Mise en scène dans un dossier temporaire + écriture du `manifest.json`.
4. Archivage `.tar.gz` vers le chemin de sortie.

Aucune mutation de la source à aucun moment.

## 7. Flux import + remap

1. Ouvrir l'archive, lire le `manifest`.
2. **Construire la table de remap** `cwd_source → cwd_cible` :
   - proposition automatique par substitution du préfixe home (ancien → nouveau) ;
   - confirmable/modifiable via prompt CLI, formulaire TUI, ou fichier `--map ancien=nouveau`.
3. Calculer les modifications :
   - **ré-encoder** chaque nouveau `cwd` → nouveau nom de dossier de projet ;
   - **réécrire le champ `cwd`** dans chaque ligne `.jsonl` (+ chaînes absolues préfixées
     par l'ancien home, de façon conservatrice) ;
   - signaler (best-effort) les chemins absolus dans config/hooks/MCP.
4. **Dry-run** : produire un rapport complet (N projets, M sessions, K réécritures,
   conflits) **sans rien écrire**.
5. **Appliquer** :
   - **backup horodaté** du `~/.claude` cible ;
   - fusion **sans écraser** l'existant (sessions clé = UUID ; conflit → **skip** par défaut,
     `--overwrite` possible) ;
   - écriture fichier par fichier via **temp + rename**.
6. Rapport de synthèse.

### Fiabilité du remap
Chaque ligne est parsée en `Value`, le champ `cwd` est modifié, puis re-sérialisée. Comme
Claude Code lit le `.jsonl` **sémantiquement** (serde), l'ordre des clés / le formatage n'a
pas d'impact fonctionnel. Des **golden tests** (entrée → sortie figée) verrouillent ce
comportement.

## 8. Gestion des erreurs

`claudine-core` renvoie `Result<T, CoreError>` (`thiserror`), variantes : `Io`,
`JsonParse{fichier, ligne}`, `ManifestVersion`, `RemapIncomplete{cwd}`, `Conflict`,
`BundleFormat`.

Principe : **jamais de perte silencieuse**. Une ligne `.jsonl` illisible n'est **pas
fatale** — elle est recopiée à l'octet près et signalée en avertissement ; seules les lignes
parsables sont modifiées. Chaque opération retourne un `Report` structuré (compteurs,
conflits, avertissements) imprimé par la CLI et affiché par le TUI.

## 9. Invariants de sûreté

- **Export** : lecture seule sur la source ; écriture uniquement dans l'archive de sortie.
- **Import** : backup horodaté **systématique** avant toute mutation ; **dry-run**
  disponible (aperçu par défaut dans le TUI) ; écritures **temp + rename** (jamais de fichier
  partiel) ; secrets ni lus ni écrits.
- **Transactionnel** : tout est mis en scène en temporaire, validé, puis commit ; échec en
  cours → restauration depuis le backup. Les backups ne sont **jamais** supprimés
  automatiquement.
- **Idempotence** : ré-importer le même bundle (politique skip) = no-op (hormis un nouveau
  backup).

## 10. Stratégie de tests (TDD sur le cœur)

- **Unitaires** sur de faux `~/.claude` (`tempdir`) : scan, métadonnées, cas limites
  d'encodage/décodage (chemins contenant des `-`).
- **Golden tests** du remap : `.jsonl` d'exemple + table de mapping → sortie attendue figée.
- **Round-trip** : export depuis home A → import dans home B avec remap → on vérifie que les
  sessions pointent sur le nouveau `cwd`, que les transcriptions sont identiques par ailleurs,
  et que les secrets sont **absents** du bundle.
- **Propriétés** : export→import→export stable ; ligne corrompue préservée.
- **CLI** via `assert_cmd` + `predicates` (snapshot du rapport de dry-run).
- **TUI** (phases 2-3) : la logique étant déjà testée dans le cœur, les écrans clés sont
  testés avec le `TestBackend` de ratatui (assertions sur le buffer).

## 11. Phasage

### Phase 1 — `claudine-core` + CLI migration (priorité)
Modèle de données, scan de `~/.claude`, `export` (bundle), `import --remap` (dry-run +
backup + fusion). Le moteur risqué, couvert par des tests. **Livrable :** migration PC
fiable et scriptable.

### Phase 2 — TUI exploration
Naviguer projets → sessions → lire une transcription ; recherche. **Livrable :** explorateur
de sessions.

### Phase 3 — TUI gestion / édition
Éditer config & mémoire, gérer MCP/plugins, ménage (supprimer / déplacer des sessions), et
exposer la migration dans le TUI. **Livrable :** panneau de contrôle complet.

## 12. Hors périmètre (YAGNI)

- Pas de synchronisation cloud / multi-machine en temps réel (l'export/import suffit).
- Pas d'édition du contenu des transcriptions (lecture seule sur les `.jsonl` côté TUI ;
  seul le remap modifie le champ `cwd`).
- Pas de gestion des secrets/credentials (volontairement exclus de la migration).
- Pas de packaging/distribution multi-plateforme en phase 1 (build local `cargo` suffit).

## 13. Risques & points ouverts

- **Encodage des chemins** : confirmer le décodage exact en lisant le `cwd` interne plutôt
  que le nom de dossier (déjà acté ci-dessus).
- **Champs porteurs de chemins** dans les `.jsonl` autres que `cwd` (ex. chemins absolus dans
  des outils) : réécriture **conservatrice** (préfixe ancien home uniquement), reste signalé.
- **Import de la config** : politique conservatrice à affiner en phase 1 (fusion fine vs
  fichier `.imported` à diffuser) — à trancher au moment de l'implémentation.
