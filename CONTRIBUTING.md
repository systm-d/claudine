# Contribuer à Claudine

Merci de l'intérêt porté au projet ! Ce guide couvre la mise en place de
l'environnement, les conventions de code et le processus de contribution.

---

## Prérequis

- Rust stable ≥ 1.74
- `git`

---

## Build & tests

```sh
# Compiler le workspace
cargo build --workspace

# Lancer tous les tests
cargo test --workspace

# Linter (zéro warning exigé)
cargo clippy --workspace -- -D warnings
```

> **Important :** ce projet est formaté à la main.
> N'exécutez **pas** `cargo fmt` — le formattage automatique n'est pas utilisé
> et modifierait des fichiers sans raison.

---

## Conventions de commit (Conventional Commits)

Les messages de commit suivent [Conventional Commits](https://www.conventionalcommits.org/).
Exemples observés dans le projet :

```
feat(tui): raccourcis Extensions (Enter hooks, p plugins) + aide
fix(corbeille): lister par entrée supprimée, pas par .jsonl
chore(sdd): phase 2a terminée + revue
docs(plan): phase 2a — édition hooks + bascule plugins (8 tâches TDD)
```

Format général : `<type>(<scope>): <description courte>`

Types courants : `feat`, `fix`, `refactor`, `test`, `docs`, `chore`.

---

## Workflow Pull Request

1. Créez une branche depuis `main` :

   ```sh
   git switch main
   git switch -c feat/ma-fonctionnalite
   ```

2. Développez en TDD si possible (`claudine-core` est la lib sans UI, testée
   unitairement ; `claudine` contient le TUI et les tests d'intégration CLI).

3. Avant d'ouvrir une PR, vérifiez :

   ```sh
   cargo test --workspace    # tous les tests passent
   cargo clippy --workspace -- -D warnings   # zéro warning
   ```

4. Ouvrez une PR vers `main`. Décrivez le changement, ses motivations et les
   tests ajoutés.

---

## Structure du projet

```
crates/
  claudine-core/   # lib pure (sans UI), testée unitairement
  claudine/        # binaire : CLI (clap) + TUI (ratatui)
docs/
  superpowers/     # specs et plans des phases
```

---

## Licence

En contribuant, vous acceptez que vos contributions soient publiées sous la
double licence **MIT OR Apache-2.0** du projet.
