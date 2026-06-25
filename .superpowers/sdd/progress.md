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
