+++
[extra]
tagline = "Gère tes données Claude Code, sans quitter le terminal."
lede = "Sessions, mémoire, configuration, extensions et marketplaces — un TUI Rust qui lit et écrit ~/.claude en toute sûreté."
cta = "Voir sur GitHub"
cta2 = "Installer"
+++

<section class="features">
  <h2>Ce que fait Claudine</h2>
  <div class="grid">
    <div class="card"><h3>Sessions &amp; projets</h3><p>Parcours, recherche, déplace, restaure les sessions de toutes tes homes.</p></div>
    <div class="card"><h3>Mémoire</h3><p>Consulte la mémoire utilisateur (CLAUDE.md) directement dans le terminal.</p></div>
    <div class="card"><h3>Configuration</h3><p>Édite settings.json avec écriture atomique et sauvegarde horodatée.</p></div>
    <div class="card"><h3>Extensions</h3><p>Hooks, serveurs MCP et plugins : lecture, édition, bascule.</p></div>
    <div class="card"><h3>Marketplaces</h3><p>Ajoute des marketplaces et installe des plugins depuis le catalogue.</p></div>
    <div class="card"><h3>Import / Export</h3><p>Bundles .tar.gz signés, remap de chemins, dry-run, exclusion des secrets.</p></div>
  </div>
</section>

<section id="install" class="install">
  <h2>Installation</h2>

```
# Depuis les sources
cargo install --git https://github.com/systm-d/claudine claudine

# Debian / Ubuntu
sudo dpkg -i claudine_*_amd64.deb

# Fedora / RHEL
sudo rpm -i claudine-*.rpm

# Arch (AUR)
yay -S claudine

# Homebrew
brew tap systm-d/claudine https://github.com/systm-d/claudine
brew install claudine
```

</section>
