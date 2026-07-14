# Claudine — Phase 1 : progression (subagent-driven)

Plan: docs/superpowers/plans/2026-06-20-claudine-phase1-migration.md
Branche: claudine-phase1

## Tâches
- Task 1: scaffolding — pending
- Task 2: error/Report — pending
- Task 3: ClaudeHome — pending
- Task 4: encode_cwd — pending
- Task 5: model + scan + testkit — pending
- Task 6: manifest — pending
- Task 7: export — pending
- Task 8: remap — pending
- Task 9: read_manifest + dry_run — pending
- Task 10: apply import — pending
- Task 11: CLI — complete

## Findings (Minor) à revoir au review final
(aucun pour l'instant)

## Completed
Task 1: complete (commits c52cee2..33ea3aa, review clean)
Task 2: complete (commits 33ea3aa..9b2d0c4, review clean; Minor: CoreError sans PartialEq — ok, plan pattern-matche)
Task 3: complete (commits 9b2d0c4..8cbfb00, review clean après fix teardown-on-panic du test env)
Task 4: complete (commits 8cbfb00..cfd73d8, review clean)
Task 5: complete (commits cfd73d8..5ebb820, review clean; Minor notés: size=byte-len, cwd projet=1re session triée, lignes vides hors message_count — tous défendables)
Task 6: complete (commits 5ebb820..cec9e7b, review clean)
Task 7: complete (commits cec9e7b..ebb5238, review clean après fix mtime+hostname; exclusion secrets vérifiée)
Task 8: complete (commits ebb5238..4d60653, review clean après ancrage explicite + tests cas-limites; Minor backlog: format!() par règle dans apply_to_path)
Task 9: complete (commits 4d60653..24463fe, review clean après retrait allow(dead_code) + if-let; ⚠️ path_rewrites_planned par-session adjugé correct)
Task 10: complete (commits 24463fe..ab0904b, re-review opus clean; faille HIGH tar-slip corrigée + newline + backup testé + apply ré-exporté)
Task 11: complete (commit a1db717, 2 CLI integration tests pass, 21 core tests pass, 0 clippy warnings)
Task 11: complete (commits ab0904b..e1db2c1, review clean; +test parse_maps)
TUI: complete (commit b6e719c — ratatui, browse/transcript/mémoire/config/export; Cargo.lock versionné)
Multi-home: complete (commits 8e52fb7, 02f194d, f1074e8) — discover_homes + config Claudine (~/.config/claudine/config.json) + CLI --home/homes add|remove + sélecteur TUI (H, ajout/retrait). 53 tests verts.
Settings form: complete (commits 524dc81 cœur, b35a886 TUI) — section Config éditable via formulaire, préserve clés inconnues, bascule JSON brut. 62 tests verts, clippy clean.
Vue agrégée: complete (commit ci-dessus) — « Tous les homes » dans le sélecteur, projets fusionnés + étiquetés. 63 tests verts.
Édition externe: complete (commit ci-dessus) — E ouvre CLAUDE.md/settings.json dans $EDITOR (suspend/restaure le TUI). 64 tests verts.
Ménage sessions: complete (commits 464ce7d cœur, ci-dessus TUI) — supprimer (corbeille+confirm) / déplacer (remap). 70 tests verts.
Agrégé par défaut: complete (commit ci-dessus) — au démarrage, tous les homes visibles si 2+. 70 tests verts.
Regroupement par home: complete (commit ci-dessus) — en-têtes par home + pagination Browse. 70 tests verts.
Chemins projets: complete (commit ci-dessus) — décodage par sondage FS pour les projets sans session. 72 tests verts.
Chemins lisibles: complete (commit ci-dessus) — ~ + troncature gauche selon largeur. 72 tests verts.
Recherche + h minuscule: complete (commit ci-dessus) — / cherche dans chemin/id/contenu, h ouvre les homes. 75 tests verts.
Restauration corbeille: complete (commit ci-dessus) — c ouvre la corbeille, Enter/r restaure. 77 tests verts.
Purge corbeille: complete (commit ci-dessus) — d supprime déf., x vide tout, confirm o/n. 81 tests verts.
Recherche live: complete (commit ci-dessus) — filtre chemin/id à la frappe, Tab=contenu. 82 tests verts.
Groupes repliables: complete (commit ci-dessus) — Espace replie/déplie un home, z tout. 83 tests verts.
Import TUI: complete (commit ci-dessus) — i: saisie chemin, aperçu dry_run, w écraser, Entrée applique. 85 tests verts.
Section Extensions: complete (commit ci-dessus) — touche 4, hooks/plugins/MCP en lecture. 90 tests verts.
Suppr. projet: complete (commit ci-dessus) — d sur Projets supprime le projet entier (corbeille), corrige ~ (0 sess.). 93 tests verts.
Fix corbeille: complete (commit ci-dessus) — entrées par dossier supprimé (projets vides visibles/restaurables). 93 tests verts.
Task 2a.1: complete (commits a572f98..22651e7, review clean) — Minor: test positionnel (extensions.rs ~150) à trier en revue finale.
Task 2a.2: complete (commits 22651e7..5a650e0, review clean) — Minor: doc de module extensions.rs encore « lecture seule » → MAJ en Task 8.
Task 2a.3: complete (commits 5a650e0..0ba7c2c, review clean). Cœur (1-3) terminé.
Task 2a.4: complete (commits 0ba7c2c..a01b542, review clean) — Minor: apply_delete précond field_idx>=2 (sûre via delete_current) ; HookEdit déjà Debug-less ; allow(dead_code) à retirer en Task 6.
Task 2a.5: complete (commits a01b542..60d9128, review clean) — Minor: 4 méthodes sans doc-comment.
Task 2a.6: complete (commits 60d9128..949495c, review clean) — éditeur hooks câblé (Enter), allow(dead_code) retiré. Minor cosmétiques (let non initialisé, buf.clone).
Task 2a.7: complete (commits 949495c..50c1285, review clean) — modal plugins (p). Minor: garde redondante, écriture partielle si erreur (accepté).
Task 2a.8: complete (commits 50c1285..22875cd, review clean) — raccourcis Extensions + doc module corrigée. PHASE 2a TERMINÉE (8/8).
Revue finale (e7a90fa..22875cd) : « avec correctifs ». Bloquant #1 : validation §8 absente (commande/évènement vide). Minors triés = acceptables. Fix en cours.
Fix revue finale: complete (commit 5c28717) — validation §8 (évènement/commande non vides) + nits. 103 tests verts, clippy clean. PHASE 2a OK pour merge.

--- PHASE 2b (branche claudine-phase2b) ---
Task 2b.1: complete (commits 4505ff3..b46dbbf, review clean) — modèle MCP + read. Minor: doc module extensions.rs « MCP hors périmètre » à MAJ en Task 6.
Task 2b.2: complete (commits b46dbbf..5ffbe5d, review clean) — write_user_mcp_servers (préserve les autres clés). Cœur 2b OK.
Task 2b.3: complete (commits 5ffbe5d..07320b1, review clean) — McpEditor navigation serveurs. Minor: pas de Debug derive.
Task 2b.4: complete (commits 07320b1..60529e4, review clean) — édition champs MCP + validation. Déviation current_value (pas de "=" pré-rempli) validée.
Task 2b.5: complete (commits 60529e4..26a3ce6, review clean) — câblage MCP (m), cohabitation m gérée, allow(dead_code) retiré. 4 risques nommés OK.
Task 2b.6: complete (commits 26a3ce6..58dad24, review clean) — raccourci m + doc module + aide. PHASE 2b 6/6.
Revue finale 2b (632ee6f..58dad24) : « ready to merge: yes ». Fix: derive Debug McpEditor (1 ligne). 115 tests verts.

--- PHASE 2c-1 (branche claudine-phase2c1) — Marketplaces & socle ---

Plan: docs/superpowers/plans/2026-06-23-claudine-phase2c1-marketplaces.md
Base branche: d588a72 (spec + plan commités)

## Tâches 2c-1
- Task 1: cœur — modèle/parse/lecture registre+manifeste/iso8601 — complete
- Task 2: cœur — git helper + add/remove/update — complete
- Task 3: TUI — état MarketplacesManager — complete
- Task 4: TUI — câblage app + concurrence (thread) + routage — complete
- Task 5: TUI — rendu modal + footer/aide + vérif finale — complete

## Findings (Minor) 2c-1 à revoir au review final
- T1: `read_marketplace_manifest` ne valide pas `name` via `is_safe_name` avant le join (chemin read, risque limité) — à corriger quand l'entrée devient user-controlled (Task 2 utilise déjà is_safe_name ailleurs).
- T1: `read_marketplaces` suppose que `SettingsDoc::load` ne `Err` pas sur fichier absent (test absent_is_empty couvre le runtime).
- T1: `is_safe_name` redondance `name != ".."` vs `!name.contains("..")` (cosmétique).
- T2 (Minor, plan-mandated): `update_marketplace` no-op silencieux si l'entrée registre est absente alors que le dossier existe (désync) — comportement hérité du plan, à trancher au review final.
- T3 (Minor): `MarketplacesManager` sans `#[derive(Debug)]` (incohérent avec McpEditor/MktMode) — à ajouter au review final.
- T3 (Minor, info): `set_items` ne reset pas `confirm_remove`/`mode` — OK en pratique (câblage T4 les remet à plat avant).
- T4 (Minor): `Event::Resize` non re-dessiné explicitement dans la branche poll (redessin auto au tick ≤120 ms, impact nul) — pourrait être géré explicitement au review final.
- T4 (Minor, info): annotation `Deferred` = style ; `confirm_remove` non gardé par `!busy` mais synchrone (pas de race).
- T5 (Minor): `url.clone()` superflu dans le `format!` de `render_marketplaces` (chemin de rendu, coût négligeable) — `url.as_str()` suffirait.

## Sécurité (à corriger dans la vague de fix Task 2)
- [HIGH] Argument injection dans `mod git::clone` : `url` commençant par `-` (flag smuggling) et transport `ext::` (exec arbitraire). Fix : rejeter url débutant par `-`, insérer `--` avant url/dest, et `-c protocol.ext.allow=never`. La source est l'utilisateur (sa machine) mais l'intention = dépôt git → défense en profondeur justifiée.

## Completed 2c-1
Task 1: complete (commit 1e05f5b, review clean — Approved) — modèle + parsing + lecture registre/manifeste + iso8601. Algo civil_from_days vérifié arithmétiquement par le reviewer. 6 tests pass, 69 full suite pass, 0 clippy. 3 Minor consignés ci-dessus.
Task 2: complete (commits 4d8bbb4 impl + f237a77 fix sécurité, review+re-review clean — Approved) — git helper + add/remove/update. HIGH arg-injection neutralisé (guard `-`, `--`, protocol.ext.allow=never) + nettoyage tmp sur échec clone. 14 tests marketplaces, suite verte, 0 clippy. Minor update-no-op consigné.
Task 3: complete (commit c86b657, review clean — Approved) — état MarketplacesManager (nav bornée, add-input, confirm remove). 4 tests, 54/54 -p claudine, 0 clippy. 2 Minor consignés (Debug derive, set_items reset).
Task 4: complete (commit 4ffaece, review clean — Approved) — câblage App + jobs git en arrière-plan (thread+mpsc), event::poll quand job actif, routage handle_marketplaces_key, ouverture g. Borrow safety + ownership thread vérifiés. 4 tests, 60 tests -p claudine, 0 clippy. 3 Minor consignés.
Task 5: complete (commit ab39535, review clean — Approved) — rendu render_marketplaces (3 états + spinner), footer/aide (g), allow(dead_code) MktJob.label retiré. Workspace: 137 tests verts, 0 clippy. 1 Minor (url.clone). PHASE 2c-1 5/5.
Revue finale (opus, e153d47..ab39535) : « Ready to merge: Yes », 0 Critical/Important. Sécurité anti-injection validée bout-en-bout, atomicité + concurrence saines. Fix recommandé T2 (update no-op→erreur) + T3 (derive Debug).
Fix wave finale: complete (commit a7ed62b, re-review Approved) — update_marketplace erre sur désync (check avant pull) + derive Debug MarketplacesManager + test. Workspace 138 tests verts, 0 clippy. PHASE 2c-1 TERMINÉE, prête pour merge.
PHASE 2c-1 mergée: PR #5 → main (merge e624e98), branche supprimée, main resync. 138 tests verts post-merge.

--- PHASE 2c-2a (branche claudine-phase2c2a) — Catalogue de plugins + désinstallation ---

Plan: docs/superpowers/plans/2026-06-23-claudine-phase2c2a-catalogue.md
Base branche: f9aa322 (spec + plan commités)

## Tâches 2c-2a
- Task 1: cœur — read_installed_plugins (exposé) + uninstall_plugin — complete
- Task 2: TUI — état PluginCatalog (2e niveau) — complete
- Task 3: TUI — câblage catalogue (app + routage) — complete
- Task 4: TUI — rendu catalogue + aide + vérif finale — complete

## Findings (Minor) 2c-2a à revoir au review final
- T2: `allow(dead_code)` ajoutés sur PluginCatalog/CatalogEntry/catalog (câblage Tasks 3-4). Stratégie : les laisser jusqu'à Task 4 (qui consomme `description` via le rendu), puis Task 4 retire les 4 et vérifie clippy 0 warning. (Un allow sur du code utilisé est inoffensif, pas de warning.)
- T2 (Minor): un `use claudine_core::{...}` placé après les helpers dans `mod tests` (hygiène) — cosmétique.
- T3 (Minor, info): `catalog_close` sans doc-comment ; write-back toggle par `find(name)` (sûr) ; reset confirm_uninstall inline (asymétrie de style). Cosmétiques.

## Sécurité 2c-2a (à corriger dans la vague de fix Task 1)
- [MEDIUM] Path traversal dans `uninstall_plugin` : `path.starts_with(cache_root)` est lexical → un `installPath` avec `..` (ex. `<cache>/../../x`) passe le garde-fou puis serait supprimé hors cache. Fix : rejeter tout composant `..` (lexical) + canonicaliser les deux chemins avant comparaison (neutralise symlinks). Pertinent car 2c-2b écrira `installPath`.

## Completed 2c-2a
Task 1: complete (commits b74d6dd impl + e118813 fix sécurité, review+re-review clean — Approved) — read_installed_plugins (wrapper sur read_plugins) + uninstall_plugin (cache confiné). MEDIUM path traversal neutralisé (rejet `..` lexical + canonicalisation symlinks). 5 tests uninstall, 83/83 crate, 0 clippy.
Task 2: complete (commits b1ecb9d impl + ce1e223 fix dead_code, review clean — Approved) — état PluginCatalog/CatalogEntry + champ catalog. Clé `<nom>@<mkt>` vérifiée. allow(dead_code) bornés (retrait en Task 4). 3 tests, 61 -p claudine, 0 clippy. 1 Minor (use placement).
Task 3: complete (commit 8b02502, review clean — Approved) — câblage catalogue (open_catalog/toggle_enable/uninstall_confirmed/close) + handle_marketplaces_key (Enter ouvre, niveau catalogue). Borrow discipline OK. 2 tests, 0 clippy. 3 Minor cosmétiques.
Task 4: complete (commit 1c6496d, review clean — Approved) — rendu render_plugin_catalog (états + confirm) + aide + retrait des 4 allow(dead_code). Workspace 148 tests verts, 0 clippy. PHASE 2c-2a 4/4.
Revue finale (opus, e624e98..1c6496d) : « Ready to merge: Yes », 0 Critical/Important. Reuse sans duplication, clé cohérente, suppression cache durcie (..+canonicalisation) vérifiée. Findings Minor/cosmétiques uniquement → merge non bloqué.

## Backlog 2c-2b (issu du review final 2c-2a)
- M1: dans `uninstall_plugin`, supprimer le dossier cache APRÈS les écritures registre (rendre le registre autoritaire) — réduit la fenêtre d'entrée pendante si save échoue. Reviewer: « acceptable tel quel », à faire en 2c-2b (qui écrira installPath).
- Cosmétiques 2c-2a non bloquants: `use` dans mod tests (marketplaces.rs) ; doc-comment `catalog_close`.

PHASE 2c-2a TERMINÉE, prête pour merge.

--- PHASE 2c-2b (branche claudine-phase2c2b) — Installation de plugins ---

Plan: docs/superpowers/plans/2026-06-23-claudine-phase2c2b-installation.md
Spec: docs/superpowers/specs/2026-06-23-claudine-phase2c2b-installation-design.md
Base branche: e4a96e7 (spec + plan commités)

## Tâches 2c-2b
- Task 1: cœur — PluginSource + parsing du `source` (4 types) — complete
- Task 2: cœur — git clone_full + checkout (commit épinglé) — complete
- Task 3: cœur — install_plugin relative-path + registre + auto-activation — complete
- Task 4: cœur — install_plugin branche git (clone+checkout+sous-dossier+temp) — complete
- Task 5: cœur — M1 : uninstall_plugin supprime le cache après les écritures registre — complete
- Task 6: TUI — touche `i` (install), job de fond + spinner, refresh entrée — complete

## Completed 2c-2b
Task 1: complete (commit f27195d, review clean — Approved) — PluginSource enum + champ source: Option<PluginSource> + parse_plugin_source (4 formes : url/git-subdir→sha, github→commit+url, relative string). Entrées à source inconnue conservées (source None, catalogue non régressé). Re-export lib.rs + fix helper pm() TUI. 84 tests workspace, 0 clippy.
  Minor (T1, pour triage review final) : (a) branche empty-string de parse_plugin_source non testée directement ; (b) `github` ignore silencieusement un éventuel `path` — ajouter un commentaire d'intention. Aucun Critical/Important.
Task 2: complete (commit eb77be4, review clean — Approved) — git::clone_full (historique complet, durci) + git::checkout(--detach), réutilisent finish(). allow(dead_code) sur les 2 (retrait Task 4). Test offline 2 commits (pin sha1 sur contenu) + bad-commit/dash-url/dash-commit → Err. 85 tests, 0 clippy.
  Minor (T2, triage final) : label finish "git clone" identique pour clone et clone_full (messages d'erreur indistinguables). Aucun Critical/Important.
Task 3: complete (commit 1360d84, review clean — Approved) — install_plugin (branche relative-path) + copy_dir_recursive (ignore symlinks) + read_plugin_version (fallback "unknown") + écriture installed_plugins.json (entrée user, autres scopes préservés) + auto-activation via extensions::set_plugin_enabled. Branche Git = Err provisoire (Task 4). Confinement double (rejet .. + starts_with). 89 tests, 0 clippy.
  Minor/obs (T3, triage final) : installedAt écrasé à la réinstallation (perte de la date d'install d'origine) — acceptable (doc « réécrit la version »). Aucun Critical/Important.
Task 4: complete (commit 34d18d6, review clean — Approved) — branche Git d'install_plugin (temp_git_<nanos>, clone_full+checkout, sous-dossier confiné, nettoyage temp sur CHAQUE retour d'erreur + après copie). allow(dead_code) retirés des 2 helpers. Tests offline (repo local en url) : url+pin+auto-enable+pas de temp résiduel, git-subdir, bad-commit→Err sans registre ni temp. 157 tests workspace, 0 clippy.
  Minor (T4, triage final) : test bad-commit passait en RED trivialement (stub renvoyait Err) — un test « clone ok / checkout échoue / temp supprimé » isolerait mieux ; chemin de nettoyage couvert en GREEN. Aucun Critical/Important.
Task 5: complete (commit 5961f59, review clean — Approved) — M1 : uninstall_plugin réordonné (valide le chemin tôt sans supprimer → écrit installed_plugins.json → écrit settings.json → supprime le cache en dernier). Hardening conservé (.. + starts_with + canonicalisation). Test unix : plugins/ en lecture seule → save échoue, cache préservé (perms restaurées avant assert). 158 tests, 0 clippy.
  Minor (T5, triage final) : commentaire au-dessus du filtre `remaining` moins précis qu'avant (perd le « quoi »). Aucun Critical/Important.
Task 6: complete (commit 1ac8bcb, review clean — Approved) — TUI : PluginCatalog::mark_installed (+test) ; MktJobKind{Marketplace,InstallPlugin} + champ kind ; tick_mkt_job branché sur kind (capture kind avant clear) ; catalog_install (job de fond, no-op si occupé) ; touche `i` + gate !busy sur Espace/i/d ; render_plugin_catalog reçoit le job (spinner + hint) ; aide MAJ. 159 tests, 0 clippy. PHASE 2c-2b 6/6.
  Minor (T6, triage final) : (a) hint "installation en cours" affiché pour tout job pendant que le catalogue est ouvert (cas rare ; mot générique « opération en cours » plus sûr) ; (b) catalog_install : clones verbeux (cosmétique). Aucun Critical/Important.

## Revue finale 2c-2b (opus, 1d04206..1ac8bcb)
Verdict : « Ready to merge: No (with fixes) » — 1 Critical, 1 Important.
- [C1 CRITICAL] install_plugin : `version` (issu d'un plugin.json tiers, non fiable) composé dans `dest=cache/<mkt>/<plugin>/<version>` sans validation ; garde `starts_with(cache_root)` purement lexical → `version="../../../../x"` s'évade → remove_dir_all + copy hors cache (primitive delete+write). Fix : valider `version` (is_safe_name → repli "unknown") + scan composant ParentDir sur dest (comme uninstall_plugin).
- [I1 IMPORTANT] confinement relative-path/git src lexical ; `is_dir()` suit les symlinks → un src symlink fait copier des fichiers externes dans le cache. Fix : canonicaliser src + re-vérifier la containment sous mkt_dir (relative) / temp (git).
- 8 Minors (T1a/T1b/T2/T3/T4/T5/T6a/T6b) tous triés « Defer » par la revue.
Points forts confirmés : durcissement git réutilisé, discipline de nettoyage temp, M1 correct, intégrité registre (array-par-scope, SettingsDoc atomique), concurrence saine, clippy 0 / 159 tests.

## Fix wave finale 2c-2b: complete (commit 424bf0a, re-review Approved)
C1 + I1 corrigés : version sanitisée via is_safe_name (repli "unknown") + rejet composant ParentDir sur dest ; canonicalisation src+racine (relative→mkt_dir, git→temp) avec rejet hors-racine et nettoyage temp préservé. 2 tests de régression (malicious version → unknown/ + rien hors cache ; symlink source #[cfg(unix)] → Err). Workspace 95 (core)/159, 0 clippy. Re-review opus-finding : C1✅ I1✅, 0 Critical/Important restant (1 Minor doc-coverage non exploitable).
PHASE 2c-2b TERMINÉE, prête pour merge (branche claudine-phase2c2b, e4a96e7..424bf0a).

## Chantier — Alignement claudine sur le template + josephine (2026-07-13, branche claudine-template-alignment)
Base branche = 55c237d (après specs/plan). Plan : docs/superpowers/plans/2026-07-13-claudine-template-alignment.md
Task 1: complete (commit 6123c1a, review clean — Approved) — edition 2024/MSRV 1.85, [workspace.dependencies]/[workspace.lints unsafe_code=forbid + clippy all=warn]/[profile.release lto+strip], rust-toolchain.toml + rustfmt.toml, crates en workspace deps + [lints]. Conflit edition-2024/forbid (set_var/remove_var → unsafe) résolu SANS déroger : extraction helpers purs config_path_from / ClaudeHome::discover_from (comme josephine, pas d'env muté en test, pas de dep). 162 tests, 0 clippy. metadata deb/rpm préservés. Pas de cargo fmt (→ T2).
Task 2: complete (commit f08c5dc, review clean — Approved) — cargo fmt unique, 21 fichiers .rs reformatés (edition 2024, max_width 100). fmt --check idempotent, 162 tests verts. Minor : la ligne ledger T1 a été happée dans ce commit (inoffensif ; ledger désormais committé à part).
Task 3: complete (commit 1747b71, review clean — Approved) — tui/* (7 fichiers) déplacés dans claudine-core via git mv (renames 97-99%), imports claudine_core::→crate:: (zéro résidu), ratatui déplacé côté core, lib.rs pub mod tui;, main.rs → claudine_core::tui::run(). Cargo.lock v3→v4 (bénin, MSRV 1.85). 162 tests (156 core dont TUI), 0 clippy, fmt clean.
Task 4: complete (commit 5791871, review clean — Approved, reviewer a reconstruit en worktree + rejoué 162/162) — CLI dans le core : cli.rs (Parser+dispatch) + commands/{mod,export,import,homes}.rs (1/sous-cmd), lib.rs mod cli/commands (privés) + pub fn run()->ExitCode. Frontière anyhow (map_err(anyhow::Error::msg), eprintln "Erreur : {e:#}") → préfixe FR + code 1 préservés. Binaire = shim ; ancien cli.rs supprimé ; deps binaire = claudine-core seul (clap+serde_json retirés) ; metadata deb/rpm intacts. 4 tests unit relocalisés. 162 tests, 0 clippy. PHASE A (fondation+restructuration) TERMINÉE.
Task 5: complete (commit 4d34081, review clean — Approved) — deny.toml/tarpaulin.toml/.cargo/audit.toml adaptés de josephine. Exclusions tarpaulin sur vrais chemins claudine (core/tui, cli.rs, commands/*, main.rs). Ignore RUSTSEC-2024-0436 (paste, non maintenu, via ratatui) légitime. cargo deny/audit/tarpaulin tournent : deny ok, audit ok, 89% cov.
  >> À FAIRE EN T8 (Important, vérifié par le reviewer) : ajouter `version.workspace = true` au path-dep `claudine-core` dans crates/claudine/Cargo.toml → supprime le workaround `[bans] skip=[{crate="claudine"}]` de deny.toml (verify `cargo deny check bans` clean) ET requis pour la publication crates.io.
  >> À FAIRE EN T8 (Minor) : corriger le commentaire d'ignore RUSTSEC-2024-0436 dans deny.toml + .cargo/audit.toml : « paste » est un dep DIRECT de ratatui (pas « via lru »).
Task 6: complete (commit 52ff24d, review clean — Approved) — CONVENTIONS.md/CLAUDE.md/AGENTS.md adaptés de josephine (multi-OS, structure core+shim, edition2024/forbid/fmt gate, sans SQLite/daemon/i18n). Table « où changer quoi » re-dérivée des vrais modules. Zéro résidu josephine, docs-only.
  >> À FAIRE EN T12 (Minor) : CONVENTIONS.md — MSRV 1.85 est pinné via Cargo.toml (rust-version), pas rust-toolchain.toml (qui ne pin que le channel) ; corriger la formulation.
  >> À FAIRE EN T12 (Minor) : CONTRIBUTING.md dit encore « N'exécutez pas cargo fmt » — contredit la nouvelle porte fmt ; mettre à jour.
Task 7: complete (commit 669d01d, review clean — Approved) — ci.yml refondu depuis josephine : lint (fmt+clippy), test matrice 6 OS (ubuntu 22/24, fedora 40/41 via container, + windows/macos avec container:"" = pas de container, vérifié contre actions/runner PR#266), coverage (tarpaulin→Codecov continue-on-error, ubuntu), security (audit+deny, ubuntu). bench-smoke retiré. YAML OK. 2 Minors acceptés tels quels (continue-on-error job-level ; ordre step fedora hérité josephine).
  >> REMANIEMENT : les actions « path-dep version + retrait skip deny + fix commentaire RUSTSEC » sont DÉPLACÉES de T8 vers T12 (elles dépendent du bump 0.1.0 fait en T12 ; éviter un 0.0.2 figé transitoire). T8 = release.yml seul.
Task 8: complete (commit 7a424b7, review clean — Approved) — release.yml : conserve trigger v*, build multi-OS, deb/rpm, homebrew(tap)/winget existants ; AJOUTE steps de rendu homebrew(.rb)/aur(PKGBUILD) attachés à la Release + job crates-io opt-in (vars.PUBLISH_CRATES, publie claudine-core PUIS claudine). YAML OK. Cargo.toml/deny.toml intacts (→ T12).
  >> T9 IMPÉRATIF (sinon on livre du cassé) : packaging/aur/PKGBUILD doit avoir `sha256sums=('<64 zéros hex>')` (PAS 'SKIP' — l'arch/PKGBUILD existant l'utilise ; le sed ne matche que [0-9a-f]* → no-op silencieux → livraison de 'SKIP' = intégrité désactivée) et `pkgver=` en début de ligne. packaging/homebrew/claudine.rb doit avoir `url "..."` (ligne seule) et `sha256 "<64 zéros hex>"` (ou "").
  >> REVUE FINALE (Important) : doublon homebrew — tap-push (Formula/claudine.rb, documenté brew install) vs nouvel asset-de-release. Reco reviewer : garder le tap canonique. Choix de distribution à trancher.
  >> VAGUE FINALE (Important, résilience) : le step de rendu homebrew/aur dans release.yml n'a pas de garde `[ -f ... ]` → un tag avant que les fichiers existent ferait échouer TOUT le job release (binaires/deb/rpm inclus). Ajouter la garde (idiome déjà présent dans le job homebrew existant).
Task 9: complete (commit b56e5d9, review clean — Approved) — packaging/aur/PKGBUILD (git mv depuis arch, sha256sums 64 zéros hex PAS 'SKIP', pkgver=0.0.2, pas de systemd), packaging/homebrew/claudine.rb (class Claudine, sans depends_on :linux, sha256 64 zéros, test --version), .github/dependabot.yml, .github/CODEOWNERS. Formes sed vérifiées. pkgver corrigé 0.1.0→0.0.2 au passage.

## CLUSTER DE CORRECTIFS release.yml (à traiter en un lot dédié — vague finale, avant/pendant la revue finale) :
  1. [Important] job `arch` référence packaging/arch/ (supprimé par le git mv T9) → repointer sur packaging/aur/ (release.yml ~L162/164/165). NB divergence : le job arch fait un build LOCAL via git-archive (attend un tarball local) alors que le PKGBUILD AUR pointe désormais l'URL du tag GitHub — réconcilier.
  2. [Important] doublon homebrew : tap-push (Formula/claudine.rb, brew install documenté) vs nouvel asset-de-release. Décision : garder le tap canonique ; faire du template packaging/homebrew/claudine.rb la source unique (ou retirer le step asset). À trancher.
  3. [Important] steps de rendu homebrew/aur sans garde `[ -f ... ]` → un tag ferait échouer tout le job release. Ajouter la garde (idiome déjà dans le job homebrew existant).
Task 10: complete (commit 6188336, review clean — Approved) — site Zola : config.toml (base_url Pages, brand_color #d97757, repo_url), content/_index.md (features claudine réelles + install cargo/deb/rpm/aur/brew), templates base+index, sass main.scss (--brand #d97757), site/public/ gitignoré. zola build OK (rebuild par le reviewer, HTML inspecté). 2 déviations validées (bloc code fencé vs <pre> ; !important sur fond syntect).
  >> Minor optionnel (T12 ou tidy) : mettre highlight_code = false dans site/config.toml (snippets shell sans coloration utile) évite la bataille !important dans main.scss.
Task 11: complete (commit 939a6aa, review clean — Approved) — pages.yml : trigger push main site/**, workflow_dispatch, permissions pages/id-token, build (configure-pages + Zola 0.21 via taiki-e + zola build + upload site/public) + deploy (deploy-pages, env github-pages). YAML OK, zéro résidu josephine.
Task 12: complete — clôture 0.1.0. TDD : 3 tests ajoutés à cli.rs (prints_version/help_lists_subcommands/unknown_flag_fails), RED confirmé (prints_version échoue sur 0.0.2, 4 autres verts), bump Cargo.toml [workspace.package] version 0.0.2→0.1.0, GREEN (5/5 cli.rs). CHANGELOG [0.1.0] + refs de lien (Unreleased→compare v0.1.0...HEAD). README : badges CI+Pages+Release, lien site, Installation réalignée (cargo crates.io + source, deb/rpm, AUR via PKGBUILD de release, Homebrew, winget), MSRV affichée 1.85 (était 1.74, obsolète). Correctifs différés : crates/claudine/Cargo.toml claudine-core path-dep + version="0.1.0" (T5/T8) → skip [bans] claudine retiré de deny.toml, `cargo deny check bans` clean sans skip (bans ok, pas de warning unnecessary-skip). Commentaire RUSTSEC-2024-0436 corrigé dans deny.toml + .cargo/audit.toml (paste = dep DIRECTE de ratatui 0.28, pas via lru). CONTRIBUTING.md : fmt guidance inversée (cargo fmt avant commit, CI vérifie --check). CONVENTIONS.md : MSRV pinné via Cargo.toml rust-version, rust-toolchain.toml ne pin que le channel. Tidy optionnel T10 fait : highlight_code=false + retrait !important sur .install pre, zola build OK, <pre><code> sans style inline vérifié dans le HTML généré. Porte finale : fmt --check OK, clippy --workspace --all-targets -D warnings 0 warning, cargo test --workspace 165 (160 core + 5 cli.rs), build --release OK, zola build OK. `cargo run -q -p claudine -- --version` → `claudine 0.1.0`. PHASE ALIGNEMENT TEMPLATE (T1–T12) TERMINÉE.
Task 12: complete (commit 22386ab + fix ed1577e, review clean après correctif) — tests/cli.rs +3 (prints_version 0.1.0, help, unknown-flag ; 2 existants conservés) ; version workspace 0.0.2→0.1.0 ; path-dep claudine-core version=0.1.0 ; deny skip retiré (bans ok sans unnecessary-skip) ; commentaire RUSTSEC-2024-0436 corrigé (paste = dep direct de ratatui) ; CONTRIBUTING/CONVENTIONS fmt+MSRV alignés ; CHANGELOG [0.1.0] ; README badges/site/install ; highlight_code=false. Fix ed1577e : README « Développement » ne contredit plus la politique fmt + MSRV 1.74→1.85. Gate HEAD : fmt OK, 165 tests, zola OK, --version=claudine 0.1.0.

## TOUTES LES TÂCHES T1-T12 COMPLÈTES. Reste : revue finale whole-branch (opus) + vague de correctifs (cluster release.yml : arch path, doublon homebrew, garde fichier absent) + finishing-a-development-branch.
