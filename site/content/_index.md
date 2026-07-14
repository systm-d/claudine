+++
[extra]
tagline = "Gère tes données Claude Code, sans quitter le terminal."
lede = "Sessions, mémoire, configuration, extensions et marketplaces — un TUI Rust qui lit et écrit ~/.claude en toute sûreté."
cta = "Voir sur GitHub"
cta2 = "Installer"
+++

<section class="preview">
<h2>À quoi ça ressemble</h2>
<p class="section-lede">Une interface entièrement au clavier, dans le terminal — deux écrans en exemple.</p>
<div class="term-window">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">claudine — ~/.claude</span></div>
<div class="term-body">
<div class="tui">
<div class="tui-head"><span class="tui-brand">Claudine</span><span class="tabs"><span class="tab active">Projets</span><span class="tab">Mémoire</span><span class="tab">Config</span><span class="tab">Extensions</span></span><span class="tui-home">2 homes</span></div>
<div class="tui-panels">
<div class="tui-col"><div class="col-title">Projets</div><div class="row sel">▸ delfour.co/system</div><div class="row">levilainpetit.dev</div><div class="row">dotfiles</div><div class="row dim">+ 4 autres…</div></div>
<div class="tui-col grow"><div class="col-title">Sessions</div><div class="row"><span class="mark">●</span> il y a 2 h · refactor core <span class="dim">(142 msg)</span></div><div class="row">hier · fix marketplaces install</div><div class="row">il y a 3 j · logo TUI exact</div><div class="row">il y a 5 j · export bundle .tar.gz</div></div>
</div>
<div class="tui-foot"><span class="key">/</span> rechercher<span class="key">m</span> déplacer<span class="key">c</span> corbeille<span class="key">?</span> aide</div>
</div>
</div>
</div>
<div class="term-window">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">claudine — extensions</span></div>
<div class="term-body">
<div class="tui">
<div class="tui-head"><span class="tui-brand">Claudine</span><span class="tabs"><span class="tab">Projets</span><span class="tab">Mémoire</span><span class="tab">Config</span><span class="tab active">Extensions</span></span></div>
<div class="tui-panels">
<div class="tui-col grow"><div class="col-title">Hooks · 3</div><div class="row"><span class="ev">PreToolUse</span> bash → <span class="dim">./guard.sh</span></div><div class="row"><span class="ev">PostToolUse</span> edit → <span class="dim">prettier --write</span></div><div class="col-title" style="margin-top:.7rem">Plugins · 5</div><div class="row"><span class="on">✓</span> superpowers <span class="dim">@official</span></div><div class="row"><span class="on">✓</span> rtk-tools <span class="dim">@systm-d</span></div><div class="row"><span class="off">○</span> notion-mcp <span class="dim">@community</span></div></div>
<div class="tui-col"><div class="col-title">Serveurs MCP · 2</div><div class="row"><span class="mark">●</span> github <span class="dim">stdio</span></div><div class="row"><span class="mark">●</span> filesystem <span class="dim">stdio</span></div></div>
</div>
<div class="tui-foot"><span class="key">e</span> éditer<span class="key">p</span> activer / désactiver<span class="key">M</span> marketplaces</div>
</div>
</div>
</div>
</section>

<section class="features">
<h2>Ce que fait Claudine</h2>
<div class="grid">
<div class="card"><h3>Sessions &amp; projets</h3><p>Parcours, recherche, déplace et restaure les sessions de toutes tes homes.</p></div>
<div class="card"><h3>Mémoire</h3><p>Consulte la mémoire utilisateur (CLAUDE.md) directement dans le terminal.</p></div>
<div class="card"><h3>Configuration</h3><p>Édite settings.json avec écriture atomique et sauvegarde horodatée.</p></div>
<div class="card"><h3>Extensions</h3><p>Hooks, serveurs MCP et plugins : lecture, édition, activation.</p></div>
<div class="card"><h3>Marketplaces</h3><p>Ajoute des marketplaces et installe des plugins depuis le catalogue.</p></div>
<div class="card"><h3>Import / Export</h3><p>Bundles .tar.gz signés, remap de chemins, dry-run, exclusion des secrets.</p></div>
</div>
</section>

<section id="usage" class="usage">
<h2>Commandes</h2>
<p class="section-lede">L'invocation nue lance la TUI ; les sous-commandes rendent tout scriptable.</p>
<div class="term-window">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">bash</span></div>
<div class="term-body cmds">
<div class="line"><span class="prompt">$</span>claudine</div>
<div class="out">→ ouvre l'interface TUI interactive</div>
<div class="line"><span class="prompt">$</span>claudine homes add <span class="arg">~/.claude-perso</span></div>
<div class="out">Home enregistrée : ~/.claude-perso</div>
<div class="line"><span class="prompt">$</span>claudine export <span class="flag">--out</span> backup.tar.gz</div>
<div class="out">Rapport : sessions 128 · projets 12 — Bundle écrit</div>
<div class="line"><span class="prompt">$</span>claudine import backup.tar.gz <span class="flag">--map</span> /old=/new <span class="flag">--dry-run</span></div>
<div class="out">(dry-run : rien n'a été écrit)</div>
</div>
</div>
</section>

<section id="install" class="install">
<h2>Installation</h2>
<div class="term-window">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">install</span></div>
<div class="term-body cmds">
<div class="comment"># Depuis les sources — toutes plateformes</div>
<div class="line"><span class="prompt">$</span>cargo install <span class="flag">--git</span> https://github.com/systm-d/claudine claudine</div>
<div class="comment"># Debian / Ubuntu</div>
<div class="line"><span class="prompt">$</span>sudo dpkg -i claudine_*_amd64.deb</div>
<div class="comment"># Fedora / RHEL</div>
<div class="line"><span class="prompt">$</span>sudo rpm -i claudine-*.rpm</div>
<div class="comment"># Arch — AUR</div>
<div class="line"><span class="prompt">$</span>yay -S claudine</div>
<div class="comment"># Homebrew</div>
<div class="line"><span class="prompt">$</span>brew tap systm-d/claudine https://github.com/systm-d/claudine</div>
<div class="line"><span class="prompt">$</span>brew install claudine</div>
</div>
</div>
</section>
