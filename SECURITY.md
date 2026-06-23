# Politique de sécurité

## Périmètre

Claudine manipule des données locales sensibles liées à Claude Code :

- Sessions de conversation (`~/.claude/projects/`)
- Mémoire (`CLAUDE.md`)
- Configuration (`settings.json`) contenant des hooks, plugins et clés de
  configuration
- Import/export de bundles `.tar.gz`

Les vulnérabilités dans le périmètre suivant nous intéressent en priorité :

- **Tar-slip / path traversal** lors de l'import d'un bundle
- **Écriture non atomique** ou corruption de `settings.json`
- **Inclusion accidentelle de secrets** (`.credentials.json`, tokens, etc.)
  dans un bundle exporté
- **Élévation de privilèges** ou exécution de code arbitraire
- **Déni de service** (boucle infinie, consommation mémoire excessive) sur des
  archives malformées

## Versions supportées

Seule la dernière version publiée sur `main` est maintenue.

## Signalement d'une vulnérabilité

**Ne créez pas d'issue publique GitHub pour signaler une vulnérabilité de
sécurité.**

Envoyez un rapport privé par e-mail à : **contact@delfour.co**

Incluez :

1. Description de la vulnérabilité
2. Étapes de reproduction (commande, fichier de test, etc.)
3. Impact estimé
4. Suggestion de correction si vous en avez une

Nous accuserons réception sous 72 heures ouvrées et communiquerons sur le
délai de correction estimé.

## Divulgation coordonnée

Nous pratiquons la divulgation coordonnée : nous demandons un délai raisonnable
(typiquement 90 jours) avant publication publique, pour permettre la
publication d'un correctif.
