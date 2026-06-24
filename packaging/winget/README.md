# Manifestes winget

À chaque tag `v*`, le job `winget` du workflow [Release](../../.github/workflows/release.yml)
génère les manifestes winget (type *portable*, pointant sur le `.exe` autonome
de la release) et les joint à la release sous `winget-manifests.tar.gz`.

## Tester un manifeste localement

```powershell
# Télécharger et extraire winget-manifests.tar.gz depuis la release, puis :
winget install --manifest manifests\systm-d.claudine\<version>
```

## Publier sur winget (`winget install claudine`)

La première publication nécessite une PR vers le dépôt communautaire
[`microsoft/winget-pkgs`](https://github.com/microsoft/winget-pkgs). Le plus
simple est d'utiliser [`wingetcreate`](https://github.com/microsoft/winget-create) :

```powershell
winget install wingetcreate
wingetcreate update systm-d.claudine `
  --version <version> `
  --urls https://github.com/systm-d/claudine/releases/download/v<version>/claudine-windows-x86_64.exe `
  --submit
```

Une fois le package accepté, les versions suivantes peuvent être soumises
automatiquement (token GitHub avec accès à un fork de `winget-pkgs`).
