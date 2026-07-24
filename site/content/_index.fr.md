+++
[extra]
tagline = "Claude Code crée. Claudine te garde aux commandes."
lede = "Claude Code remplit ~/.claude chaque jour — des centaines de sessions, mémoire, config, plugins, serveurs MCP, usage. Claudine est l'application compagnon qui te laisse explorer, comprendre et contrôler tout ça, depuis une seule interface terminal."
cta = "Voir sur GitHub"
cta2 = "Installer"
+++

<section class="flow-section">
<p class="flow-kicker">Claude Code produit la connaissance. Claudine t'en garde le contrôle.</p>
<div class="flow" aria-label="Claude Code alimente Claudine, qui te donne une seule interface">
<div class="flow-node src"><span class="flow-name">Claude Code</span><span class="flow-sub">écrit dans ~/.claude</span></div>
<div class="flow-arrow" aria-hidden="true">▼</div>
<div class="flow-chips">
<span class="chip">sessions</span><span class="chip">mémoire</span><span class="chip">projets</span><span class="chip">config</span><span class="chip">plugins</span><span class="chip">MCP</span><span class="chip">marketplaces</span><span class="chip">usage</span>
</div>
<div class="flow-arrow" aria-hidden="true">▼</div>
<div class="flow-node hub"><span class="flow-name">Claudine</span><span class="flow-sub">une seule interface terminal</span></div>
<div class="flow-arrow" aria-hidden="true">▼</div>
<div class="flow-node you"><span class="flow-name">toi</span><span class="flow-sub">aux commandes</span></div>
</div>
</section>

<section class="why">
<h2>Pourquoi Claudine existe</h2>
<div class="why-grid">
<div class="why-text">
<p>À chaque session, Claude Code laisse des traces dans <code>~/.claude</code> : des transcripts nommés par UUID, un <code>settings.json</code> qui grossit, des fichiers mémoire, des hooks, des plugins, des serveurs MCP, des marketplaces, des sauvegardes et des relevés d'usage.</p>
<p>Ça s'accumule vite. En quelques semaines, ce sont des centaines de sessions réparties sur une douzaine de projets — aucune portant un nom que tu reconnaîtrais.</p>
<p>L'alternative, c'est <code>cat</code>, <code>grep</code>, <code>jq</code> et un éditeur, un fichier à la fois, en espérant ne pas casser le JSON. Claudine, c'est l'appli dédiée à la place : elle lit tout l'arbre, l'affiche en clair, et réécrit en toute sûreté.</p>
</div>
<div class="term-window why-tree">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">~/.claude — sans Claudine</span></div>
<div class="term-body cmds">
<div class="line dim">~/.claude</div>
<div class="line dim">├── projects/</div>
<div class="line dim">│   ├── -home-toi-delfour-system/</div>
<div class="line">│   │   ├── <span class="arg">a1b2c3d4</span>-9f…-0e11.jsonl <span class="cmt">← laquelle ?</span></div>
<div class="line">│   │   ├── <span class="arg">7f3e0c11</span>-2a…-77d3.jsonl</div>
<div class="line">│   │   └── <span class="arg">2b9dd4a6</span>-c4…-9a10.jsonl</div>
<div class="line dim">│   └── -home-toi-dotfiles/  <span class="cmt">…</span></div>
<div class="line dim">├── settings.json      <span class="cmt"># à la main ?</span></div>
<div class="line dim">├── CLAUDE.md</div>
<div class="line dim">├── plugins/ · mcp/ · statsig/</div>
<div class="line dim">└── …  <span class="cmt">128 sessions · 12 projets</span></div>
</div>
</div>
</div>
</section>

<section class="missions">
<h2>Six choses qu'elle fait pour toi</h2>
<p class="section-lede">Pas une liste de boutons — les tâches que tu ouvres vraiment.</p>
<div class="grid">

<div class="mission">
<div class="mission-glyph" aria-hidden="true">⌕</div>
<h3>Explorer</h3>
<p class="mission-line">Retrouve n'importe quelle session, tout de suite.</p>
<ul>
<li>Sessions listées par titre, pas par UUID</li>
<li>Filtre live sur nom / chemin / id</li>
<li>Recherche plein texte dans les conversations</li>
<li>Toutes les homes d'un coup, ou une seule</li>
</ul>
</div>

<div class="mission">
<div class="mission-glyph" aria-hidden="true">☰</div>
<h3>Comprendre</h3>
<p class="mission-line">Lis les conversations sans le bruit.</p>
<ul>
<li>Transcript débarrassé des métadonnées internes</li>
<li>Appels d'outils et résultats résumés</li>
<li>Horodatages condensés et lisibles</li>
<li><span class="kbd">a</span> révèle tout quand il le faut</li>
</ul>
</div>

<div class="mission">
<div class="mission-glyph" aria-hidden="true">⚙</div>
<h3>Personnaliser</h3>
<p class="mission-line">Configure Claude Code, en sûreté.</p>
<ul>
<li>Édite <code>settings.json</code> en écriture atomique</li>
<li>Sauvegarde horodatée avant chaque modif</li>
<li>Consulte la mémoire (<code>CLAUDE.md</code>) sur place</li>
</ul>
</div>

<div class="mission">
<div class="mission-glyph" aria-hidden="true">⧉</div>
<h3>Étendre</h3>
<p class="mission-line">Gère plugins, hooks et serveurs MCP.</p>
<ul>
<li>Lis hooks, plugins et serveurs MCP</li>
<li>Active / désactive les plugins</li>
<li>Ajoute des marketplaces, installe au catalogue</li>
</ul>
</div>

<div class="mission">
<div class="mission-glyph" aria-hidden="true">⛨</div>
<h3>Protéger</h3>
<p class="mission-line">Sauvegarde, restaure et migre.</p>
<ul>
<li>Bundles d'export <code>.tar.gz</code> signés</li>
<li>Remap des chemins à l'import, avec dry-run</li>
<li>Secrets exclus automatiquement</li>
<li>Corbeille récupérable — rien de perdu pour de bon</li>
</ul>
</div>

<div class="mission">
<div class="mission-glyph" aria-hidden="true">◔</div>
<h3>Observer</h3>
<p class="mission-line">Suis l'usage et le coût en tokens.</p>
<ul>
<li>Tokens entrée / sortie / cache, par modèle</li>
<li>Coût estimé par famille de modèle</li>
<li>Grille d'activité quotidienne façon GitHub</li>
<li>Détail par session à la demande</li>
</ul>
</div>

</div>
</section>

<section class="preview">
<h2>Une seule interface, entièrement au clavier</h2>
<p class="section-lede">Deux écrans du TUI — chacun pour une tâche.</p>
<figure class="shot">
<div class="term-window">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">claudine — ~/.claude</span></div>
<div class="term-body">
<div class="tui">
<div class="tui-head"><span class="tui-brand">Claudine</span><span class="tabs"><span class="tab active">Projets</span><span class="tab">Mémoire</span><span class="tab">Config</span><span class="tab">Extensions</span><span class="tab">Usage</span></span><span class="tui-home">2 homes</span></div>
<div class="tui-panels">
<div class="tui-col"><div class="col-title">Projets</div><div class="row sel">▸ delfour.co/system</div><div class="row">levilainpetit.dev</div><div class="row">dotfiles</div><div class="row dim">+ 4 autres…</div></div>
<div class="tui-col grow"><div class="col-title">Sessions</div><div class="row sel">▸ <span class="mark">refactor core</span> <span class="dim">a1b2c3d4 · 142 msg · 2026-07-22 17:24</span></div><div class="row">fix marketplaces install <span class="dim">7f3e0c11 · 88 msg · 2026-07-21 09:12</span></div><div class="row">logo TUI exact <span class="dim">2b9dd4a6 · 41 msg · 2026-07-19 18:44</span></div><div class="row">export bundle .tar.gz <span class="dim">c0771e9f · 63 msg · 2026-07-17 11:07</span></div></div>
</div>
<div class="tui-foot"><span class="key">/</span> rechercher<span class="key">m</span> déplacer<span class="key">c</span> corbeille<span class="key">?</span> aide</div>
</div>
</div>
</div>
<figcaption>Explorer — chaque session par son nom, sur toutes les homes.</figcaption>
</figure>
<figure class="shot">
<div class="term-window">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">claudine — extensions</span></div>
<div class="term-body">
<div class="tui">
<div class="tui-head"><span class="tui-brand">Claudine</span><span class="tabs"><span class="tab">Projets</span><span class="tab">Mémoire</span><span class="tab">Config</span><span class="tab active">Extensions</span><span class="tab">Usage</span></span></div>
<div class="tui-panels">
<div class="tui-col grow"><div class="col-title">Hooks · 3</div><div class="row"><span class="ev">PreToolUse</span> bash → <span class="dim">./guard.sh</span></div><div class="row"><span class="ev">PostToolUse</span> edit → <span class="dim">prettier --write</span></div><div class="col-title" style="margin-top:.7rem">Plugins · 5</div><div class="row"><span class="on">✓</span> superpowers <span class="dim">@official</span></div><div class="row"><span class="on">✓</span> rtk-tools <span class="dim">@systm-d</span></div><div class="row"><span class="off">○</span> notion-mcp <span class="dim">@community</span></div></div>
<div class="tui-col"><div class="col-title">Serveurs MCP · 2</div><div class="row"><span class="mark">●</span> github <span class="dim">stdio</span></div><div class="row"><span class="mark">●</span> filesystem <span class="dim">stdio</span></div></div>
</div>
<div class="tui-foot"><span class="key">e</span> éditer<span class="key">p</span> activer / désactiver<span class="key">M</span> marketplaces</div>
</div>
</div>
</div>
<figcaption>Étendre — hooks, plugins et serveurs MCP dans une seule vue.</figcaption>
</figure>
</section>

<section id="usage" class="usage">
<h2>Scriptable, aussi</h2>
<p class="section-lede">L'invocation nue lance le TUI. Les sous-commandes rendent la même puissance disponible pour tes scripts et ta CI.</p>
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
<div class="line"><span class="prompt">$</span>claudine update <span class="flag">--check</span></div>
<div class="out">Mise à jour disponible : 0.1.2 → 0.1.3</div>
</div>
</div>
</section>

<section id="install" class="install">
<h2>Installation</h2>
<p class="section-lede">Local-first et open source. Elle lit et écrit uniquement <code>~/.claude</code> — pas de compte, pas de télémétrie, pas de cloud.</p>
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
