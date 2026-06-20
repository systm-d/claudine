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
