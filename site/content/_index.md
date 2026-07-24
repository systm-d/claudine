+++
[extra]
tagline = "Claude Code creates. Claudine keeps you in control."
lede = "Claude Code fills ~/.claude every day — hundreds of sessions, memory, config, plugins, MCP servers, usage. Claudine is the companion app that lets you explore, understand and control all of it, from one terminal interface."
cta = "View on GitHub"
cta2 = "Install"
+++

<section class="flow-section">
<p class="flow-kicker">Claude Code writes the knowledge. Claudine keeps control of it.</p>
<div class="flow" aria-label="Claude Code feeds data into Claudine, which gives you one interface">
<div class="flow-node src"><span class="flow-name">Claude Code</span><span class="flow-sub">writes to ~/.claude</span></div>
<div class="flow-arrow" aria-hidden="true">▼</div>
<div class="flow-chips">
<span class="chip">sessions</span><span class="chip">memory</span><span class="chip">projects</span><span class="chip">config</span><span class="chip">plugins</span><span class="chip">MCP</span><span class="chip">marketplaces</span><span class="chip">usage</span>
</div>
<div class="flow-arrow" aria-hidden="true">▼</div>
<div class="flow-node hub"><span class="flow-name">Claudine</span><span class="flow-sub">one terminal interface</span></div>
<div class="flow-arrow" aria-hidden="true">▼</div>
<div class="flow-node you"><span class="flow-name">you</span><span class="flow-sub">in control</span></div>
</div>
</section>

<section class="why">
<h2>Why Claudine exists</h2>
<div class="why-grid">
<div class="why-text">
<p>Every session, Claude Code leaves things behind in <code>~/.claude</code>: transcripts named by UUID, a growing <code>settings.json</code>, memory files, hooks, plugins, MCP servers, marketplaces, backups and usage records.</p>
<p>It piles up fast. After a few weeks it's hundreds of sessions across a dozen projects — none of them named after anything you'd recognize.</p>
<p>The alternative is <code>cat</code>, <code>grep</code>, <code>jq</code> and a text editor, one file at a time, hoping you don't corrupt the JSON. Claudine is the dedicated app instead: it reads the whole tree, shows it in plain language, and writes back safely.</p>
</div>
<div class="term-window why-tree">
<div class="term-bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="term-title">~/.claude — without Claudine</span></div>
<div class="term-body cmds">
<div class="line dim">~/.claude</div>
<div class="line dim">├── projects/</div>
<div class="line dim">│   ├── -home-you-delfour-system/</div>
<div class="line">│   │   ├── <span class="arg">a1b2c3d4</span>-9f…-0e11.jsonl <span class="cmt">← which one?</span></div>
<div class="line">│   │   ├── <span class="arg">7f3e0c11</span>-2a…-77d3.jsonl</div>
<div class="line">│   │   └── <span class="arg">2b9dd4a6</span>-c4…-9a10.jsonl</div>
<div class="line dim">│   └── -home-you-dotfiles/  <span class="cmt">…</span></div>
<div class="line dim">├── settings.json      <span class="cmt"># edit by hand?</span></div>
<div class="line dim">├── CLAUDE.md</div>
<div class="line dim">├── plugins/ · mcp/ · statsig/</div>
<div class="line dim">└── …  <span class="cmt">128 sessions · 12 projects</span></div>
</div>
</div>
</div>
</section>

<section class="missions">
<h2>Six things it does for you</h2>
<p class="section-lede">Not a list of buttons — the jobs you actually reach for.</p>
<div class="grid">

<div class="mission">
<div class="mission-glyph" aria-hidden="true">⌕</div>
<h3>Explore</h3>
<p class="mission-line">Find any session instantly.</p>
<ul>
<li>Sessions listed by title, not UUID</li>
<li>Live filter on name / path / id</li>
<li>Full-text search inside conversations</li>
<li>Every home at once, or one at a time</li>
</ul>
</div>

<div class="mission">
<div class="mission-glyph" aria-hidden="true">☰</div>
<h3>Understand</h3>
<p class="mission-line">Read conversations without the noise.</p>
<ul>
<li>Transcript with internal metadata stripped</li>
<li>Tool calls and results summarized</li>
<li>Condensed, human timestamps</li>
<li><span class="kbd">a</span> reveals everything when you need it</li>
</ul>
</div>

<div class="mission">
<div class="mission-glyph" aria-hidden="true">⚙</div>
<h3>Customize</h3>
<p class="mission-line">Configure Claude Code, safely.</p>
<ul>
<li>Edit <code>settings.json</code> with atomic writes</li>
<li>Timestamped backup before every change</li>
<li>Read user memory (<code>CLAUDE.md</code>) in place</li>
</ul>
</div>

<div class="mission">
<div class="mission-glyph" aria-hidden="true">⧉</div>
<h3>Extend</h3>
<p class="mission-line">Manage plugins, hooks and MCP servers.</p>
<ul>
<li>Read hooks, plugins and MCP servers</li>
<li>Toggle plugins on and off</li>
<li>Add marketplaces, install from the catalog</li>
</ul>
</div>

<div class="mission">
<div class="mission-glyph" aria-hidden="true">⛨</div>
<h3>Protect</h3>
<p class="mission-line">Back up, restore and migrate.</p>
<ul>
<li>Signed <code>.tar.gz</code> export bundles</li>
<li>Path remapping on import, with dry-run</li>
<li>Secrets excluded automatically</li>
<li>Recoverable trash — nothing gone for good</li>
</ul>
</div>

<div class="mission">
<div class="mission-glyph" aria-hidden="true">◔</div>
<h3>Observe</h3>
<p class="mission-line">Track usage and token cost.</p>
<ul>
<li>Tokens in / out / cache, per model</li>
<li>Estimated cost per model family</li>
<li>GitHub-style daily activity heatmap</li>
<li>Per-session breakdown on demand</li>
</ul>
</div>

</div>
</section>

<section class="preview">
<h2>One interface, entirely at the keyboard</h2>
<p class="section-lede">Two screens from the TUI — each doing one job.</p>
<figure class="shot">
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
<figcaption>Explore — every session by name, across every home.</figcaption>
</figure>
<figure class="shot">
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
<figcaption>Extend — hooks, plugins and MCP servers in one view.</figcaption>
</figure>
</section>

<section id="usage" class="usage">
<h2>Scriptable, too</h2>
<p class="section-lede">Run it bare and it opens the TUI. Subcommands make the same power available to your scripts and CI.</p>
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
<h2>Install</h2>
<p class="section-lede">Local-first and open source. It reads and writes only <code>~/.claude</code> — no account, no telemetry, no cloud.</p>
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
