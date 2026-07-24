+++
[extra]
tagline = "Manage your Claude Code data without leaving the terminal."
lede = "Sessions, memory, configuration, extensions, usage stats and marketplaces — a Rust TUI that reads and writes ~/.claude safely."
cta = "View on GitHub"
cta2 = "Install"
+++

<section class="preview">
<h2>What it looks like</h2>
<p class="section-lede">A fully keyboard-driven interface, right in your terminal — two example screens.</p>
<div class="term-window">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">claudine — ~/.claude</span></div>
<div class="term-body">
<div class="tui">
<div class="tui-head"><span class="tui-brand">Claudine</span><span class="tabs"><span class="tab active">Projects</span><span class="tab">Memory</span><span class="tab">Config</span><span class="tab">Extensions</span><span class="tab">Usage</span></span><span class="tui-home">2 homes</span></div>
<div class="tui-panels">
<div class="tui-col"><div class="col-title">Projects</div><div class="row sel">▸ delfour.co/system</div><div class="row">levilainpetit.dev</div><div class="row">dotfiles</div><div class="row dim">+ 4 more…</div></div>
<div class="tui-col grow"><div class="col-title">Sessions</div><div class="row sel">▸ <span class="mark">refactor core</span> <span class="dim">a1b2c3d4 · 142 msgs · 2026-07-22 17:24</span></div><div class="row">fix marketplaces install <span class="dim">7f3e0c11 · 88 msgs · 2026-07-21 09:12</span></div><div class="row">exact TUI logo <span class="dim">2b9dd4a6 · 41 msgs · 2026-07-19 18:44</span></div><div class="row">export .tar.gz bundle <span class="dim">c0771e9f · 63 msgs · 2026-07-17 11:07</span></div></div>
</div>
<div class="tui-foot"><span class="key">/</span> search<span class="key">m</span> move<span class="key">c</span> trash<span class="key">?</span> help</div>
</div>
</div>
</div>
<div class="term-window">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">claudine — extensions</span></div>
<div class="term-body">
<div class="tui">
<div class="tui-head"><span class="tui-brand">Claudine</span><span class="tabs"><span class="tab">Projects</span><span class="tab">Memory</span><span class="tab">Config</span><span class="tab active">Extensions</span><span class="tab">Usage</span></span></div>
<div class="tui-panels">
<div class="tui-col grow"><div class="col-title">Hooks · 3</div><div class="row"><span class="ev">PreToolUse</span> bash → <span class="dim">./guard.sh</span></div><div class="row"><span class="ev">PostToolUse</span> edit → <span class="dim">prettier --write</span></div><div class="col-title" style="margin-top:.7rem">Plugins · 5</div><div class="row"><span class="on">✓</span> superpowers <span class="dim">@official</span></div><div class="row"><span class="on">✓</span> rtk-tools <span class="dim">@systm-d</span></div><div class="row"><span class="off">○</span> notion-mcp <span class="dim">@community</span></div></div>
<div class="tui-col"><div class="col-title">MCP servers · 2</div><div class="row"><span class="mark">●</span> github <span class="dim">stdio</span></div><div class="row"><span class="mark">●</span> filesystem <span class="dim">stdio</span></div></div>
</div>
<div class="tui-foot"><span class="key">e</span> edit<span class="key">p</span> enable / disable<span class="key">M</span> marketplaces</div>
</div>
</div>
</div>
</section>

<section class="features">
<h2>What Claudine does</h2>
<div class="grid">
<div class="card"><h3>Named sessions</h3><p>The list shows the session title, not the UUID — move and restore across all your homes.</p></div>
<div class="card"><h3>Live search</h3><p>Filter by name / path / id as you type, then search inside content from 3 characters — snippets centered on the match.</p></div>
<div class="card"><h3>Readable transcript</h3><p>The conversation without the internal noise; tool calls and results summarized; condensed timestamps.</p></div>
<div class="card"><h3>Memory</h3><p>Read the user memory (CLAUDE.md) right in the terminal.</p></div>
<div class="card"><h3>Configuration</h3><p>Edit settings.json with atomic writes and timestamped backups.</p></div>
<div class="card"><h3>Extensions</h3><p>Hooks, MCP servers and plugins: read, edit, enable.</p></div>
<div class="card"><h3>Marketplaces</h3><p>Add marketplaces and install plugins from the catalog.</p></div>
<div class="card"><h3>Usage stats</h3><p>Tokens and estimated cost per model, plus a GitHub-style daily activity heatmap; per-session breakdown.</p></div>
<div class="card"><h3>Import / Export</h3><p>Signed .tar.gz bundles, path remapping, dry-run, secret exclusion.</p></div>
<div class="card"><h3>Self-update</h3><p><code>claudine update</code> fetches and installs the latest GitHub release for your platform.</p></div>
</div>
</section>

<section id="usage" class="usage">
<h2>Commands</h2>
<p class="section-lede">Running it bare launches the TUI; subcommands make everything scriptable.</p>
<div class="term-window">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">bash</span></div>
<div class="term-body cmds">
<div class="line"><span class="prompt">$</span>claudine</div>
<div class="out">→ opens the interactive TUI</div>
<div class="line"><span class="prompt">$</span>claudine homes add <span class="arg">~/.claude-perso</span></div>
<div class="out">Home registered: ~/.claude-perso</div>
<div class="line"><span class="prompt">$</span>claudine export <span class="flag">--out</span> backup.tar.gz</div>
<div class="out">Report: 128 sessions · 12 projects — Bundle written</div>
<div class="line"><span class="prompt">$</span>claudine import backup.tar.gz <span class="flag">--map</span> /old=/new <span class="flag">--dry-run</span></div>
<div class="out">(dry-run: nothing was written)</div>
<div class="line"><span class="prompt">$</span>claudine update <span class="flag">--check</span></div>
<div class="out">Update available: 0.1.2 → 0.1.3</div>
</div>
</div>
</section>

<section id="install" class="install">
<h2>Installation</h2>
<div class="term-window">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">install</span></div>
<div class="term-body cmds">
<div class="comment"># From source — all platforms</div>
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
