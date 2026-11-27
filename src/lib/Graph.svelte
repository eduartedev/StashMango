<script>
  import { clampColumn, columnTemplate } from "$lib/columns.js";
  import { graphFit, laneFits } from "$lib/graphFit.js";
  import { menuPosition } from "$lib/branchMenu.js";
  import { chipKind, isCurrentRef, packRefs, refChips, refTarget } from "$lib/refChips.js";
  import { sameWorktreePath, worktreeLabel } from "$lib/worktreeCard.js";

  let {
    commits = [],
    selected = "",
    onselect,
    refsWidth = 220,
    graphWidth = 160,
    sideOpen = true,
    detailOpen = true,
    onToggle,
    onRefs,
    onGraph,
    branches = [],
    current = "",
    onpick,
    onmenu,
    fields = ["refs", "graph", "message", "date"],
    pending = 0,
    worktrees = [],
    repoPath = "",
    stashRefs = [],
    pendingConflicts = 0,
    preferences = null,
  } = $props();
  let refMenu = $state(null);
  const row = 36;
  const tones = ["#f5b942", "#e8941a", "#2ec8c0", "#e14b8c", "#8fd14f", "#6eb0d6", "#c9a0e0", "#3eaa8c", "#e07a3d", "#7dcfb6"];
  const tonesAlt = ["#4fc3f7", "#81c784", "#ffb74d", "#ce93d8", "#a1887f", "#80cbc4", "#f06292", "#ba68c8", "#4db6ac", "#7986cb"];

  let activeTones = $derived(preferences?.graphColorIndex === 1 ? tonesAlt : tones);
  let hoverBranch = $state("");
  let groupColors = $derived({
    gray: "#8c7b66",
    mango: "#f5b942",
    teal: "#2ec8c0",
    rose: "#e14b8c",
  });

  function filterRefs(refs, visibility = "all") {
    if (!refs || visibility === "all") return refs;
    if (visibility === "none") return [];
    return refs.filter((ref) => {
      const label = ref.label || ref.name || "";
      const isRemote = label.includes("/") || ref.type === "remote";
      return visibility === "local" ? !isRemote : isRemote;
    });
  }

  function tone(index) {
    return activeTones[index % activeTones.length];
  }

  let anchored = $derived(
    (worktrees || [])
      .map((tree) => {
        const isCurrent = sameWorktreePath(tree.path, repoPath);
        const count = isCurrent ? pending : tree.dirty || 0;
        if (!count) return null;
        const head = commits.find((commit) => commit.id === tree.head) || (isCurrent ? commits[0] : null);
        if (!head) return null;
        const label = isCurrent ? current || "cambios" : worktreeLabel(tree) || "cambios";
        return {
          node: {
            id: `__pending__${tree.path}`,
            pending: true,
            pendingCurrent: isCurrent,
            pendingLabel: label,
            pendingCount: count,
            pendingPath: tree.path,
            lane: head.lane,
            parents: [head.id],
            parent_lanes: [head.lane],
            refs: "",
            summary: isCurrent
              ? `${count} cambio${count === 1 ? "" : "s"} sin commitear`
              : `${label}: ${count} cambio${count === 1 ? "" : "s"} sin commitear`,
            author: "",
            time: 0,
          },
          head: head.id,
        };
      })
      .filter(Boolean)
  );

  let commitLaneCount = $derived(commits.length ? Math.max(...commits.map((commit) => commit.lane)) + 1 : 1);

  let stashAnchored = $derived(
    (stashRefs || [])
      .map((stash, index) => {
        const base = commits.find((commit) => commit.id === stash.base);
        if (!base) return null;
        return {
          node: {
            id: `__stash__${stash.id}`,
            stash: true,
            stashId: stash.id,
            lane: commitLaneCount + index,
            parents: [base.id],
            parent_lanes: [base.lane],
            refs: "",
            summary: stash.label || stash.id,
            author: "",
            time: 0,
          },
          head: base.id,
        };
      })
      .filter(Boolean)
  );

  let rows = $derived.by(() => {
    const pendingByHead = new Map();
    for (const entry of anchored) {
      const list = pendingByHead.get(entry.head) || [];
      list.push(entry.node);
      pendingByHead.set(entry.head, list);
    }
    const stashByHead = new Map();
    for (const entry of stashAnchored) {
      const list = stashByHead.get(entry.head) || [];
      list.push(entry.node);
      stashByHead.set(entry.head, list);
    }
    const out = [];
    for (const commit of commits) {
      for (const node of pendingByHead.get(commit.id) || []) out.push(node);
      for (const node of stashByHead.get(commit.id) || []) out.push(node);
      out.push(commit);
    }
    return out;
  });

  let laneCount = $derived(rows.length ? Math.max(...rows.map((commit) => commit.lane)) + 1 : 0);
  let fit = $derived(graphFit(laneCount, graphWidth));

  function x(lane) {
    return 16 + lane * fit.gap;
  }

  function y(index) {
    return index * row + row / 2;
  }

  function ago(time) {
    if (!time) return "";
    const days = Math.floor((Date.now() / 1000 - time) / 86400);
    if (days <= 0) return "hoy";
    if (days === 1) return "hace 1 día";
    if (days < 30) return `hace ${days} días`;
    const months = Math.floor(days / 30);
    if (months === 1) return "hace 1 mes";
    if (months < 12) return `hace ${months} meses`;
    const years = Math.floor(months / 12);
    return years === 1 ? "hace 1 año" : `hace ${years} años`;
  }

  function edge(x1, y1, x2, y2) {
    if (Math.abs(x1 - x2) < 1) return `M ${x1} ${y1} V ${y2}`;
    const drop = Math.min(30, Math.abs(y2 - y1) / 2.5);
    const yBend = y2 - Math.sign(y2 - y1) * drop;
    return `M ${x1} ${y1} V ${yBend} C ${x1} ${y2}, ${x2} ${yBend}, ${x2} ${y2}`;
  }

  let width = $derived(fit.width);
  let cols = $derived(columnTemplate(fields, refsWidth, width));
  const heads = { refs: "Ramas", graph: "Grafo", message: "Mensaje", author: "Autor", date: "Fecha", hash: "Hash" };
  let height = $derived(Math.max(row, rows.length * row));

  function dragSize(event, name, start, notify) {
    const startX = event.clientX;
    event.currentTarget.setPointerCapture?.(event.pointerId);
    const move = (ev) => notify?.(clampColumn(name, start + ev.clientX - startX), false);
    const up = (ev) => {
      event.currentTarget.removeEventListener("pointermove", move);
      event.currentTarget.removeEventListener("pointerup", up);
      notify?.(clampColumn(name, start + ev.clientX - startX), true);
    };
    event.currentTarget.addEventListener("pointermove", move);
    event.currentTarget.addEventListener("pointerup", up);
  }

  function openRefs(event, chips) {
    event.stopPropagation();
    const rect = event.currentTarget.getBoundingClientRect();
    const place = menuPosition(rect.left, rect.bottom + 6, 280, 220, {
      width: window.innerWidth,
      height: window.innerHeight,
    });
    refMenu = { ...place, chips };
  }

  function chooseRef(chip) {
    const name = refTarget(chip, branches);
    refMenu = null;
    if (name) onpick?.(name);
  }

  function onBranchHover(branch) {
    if (preferences?.highlightBranchRows) {
      hoverBranch = branch;
    }
  }

  function onBranchLeave() {
    hoverBranch = "";
  }

  function isRowHighlighted(commit) {
    if (!hoverBranch || !preferences?.highlightBranchRows) return false;
    return commit.refs?.some((ref) => ref.label === hoverBranch || ref.name === hoverBranch);
  }
</script>

<div class="graph" style:--cols={cols}>
  <div class="colbar">
    {#each fields as id, index (id)}
      <div class="colhead">
        {#if index === 0}
          <button type="button" class="layout" class:on={sideOpen} aria-pressed={sideOpen} title={sideOpen ? "Cerrar panel" : "Abrir panel"} onclick={() => onToggle?.("side")}>
            <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="1.5" y="2.5" width="13" height="11" rx="1.5"/><path d="M6 2.5v11"/></svg>
          </button>
        {/if}
        <span>{heads[id]}</span>
        {#if id === "refs"}
          <button type="button" class="grip" aria-label="Ancho de ramas" onpointerdown={(event) => dragSize(event, "refsWidth", refsWidth, onRefs)}></button>
        {:else if id === "graph"}
          <button type="button" class="grip" aria-label="Ancho del grafo" onpointerdown={(event) => dragSize(event, "graphWidth", width, onGraph)}></button>
        {/if}
      </div>
    {/each}
    <button type="button" class="layout edge" class:on={detailOpen} aria-pressed={detailOpen} title={detailOpen ? "Contraer detalle" : "Expandir detalle"} onclick={() => onToggle?.("detail")}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="1.5" y="2.5" width="13" height="11" rx="1.5"/><path d="M10 2.5v11"/></svg>
    </button>
  </div>
  <div class="body">
  {#if fields.includes("refs")}
  <div class="refcol">
    {#each rows as commit (commit.id)}
      {@const filteredRefs = preferences?.showBranchesInGraph ? filterRefs(commit.refs, preferences?.graphBranchVisibility) : []}
      {@const packed = packRefs(refChips(filteredRefs))}
      <div class="refcell" class:selected={selected === commit.id} class:pending={commit.pending} class:stash={commit.stash} style:--lane-color={commit.pending || commit.stash ? "#f5b942" : tone(commit.lane)} role="presentation">
        {#if commit.stash}
          <span class="label stash" title={commit.summary}>
            <span class="stash-dot"></span>
            <span class="name">{commit.stashId}</span>
          </span>
        {:else if commit.pending}
          <span class="label pending" class:alert={commit.pendingCurrent && pendingConflicts > 0} title="{commit.pendingCount} archivos sin commitear{commit.pendingCurrent && pendingConflicts ? ` · ${pendingConflicts} conflicto${pendingConflicts === 1 ? "" : "s"}` : ""}{commit.pendingCurrent ? "" : ` en ${commit.pendingPath}`}">
            <span class="pending-dot"></span>
            <span class="name">{commit.pendingLabel}</span>
          </span>
        {:else if packed.shown}
          {@const target = isCurrentRef(packed.shown, current) ? "" : refTarget(packed.shown, branches)}
          <svelte:element
            this={target || packed.extra.length ? "button" : "span"}
            type={target || packed.extra.length ? "button" : undefined}
            class="label {packed.shown.kind}"
            title={target ? `Cambiar a ${target}` : packed.shown.synced ? `${packed.shown.title} · origin/${packed.shown.label}` : packed.shown.title}
            onclick={target ? () => onpick?.(target) : packed.extra.length ? (event) => openRefs(event, refChips(commit.refs)) : undefined}
            oncontextmenu={(event) => { event.preventDefault(); onmenu?.(event, commit); }}
            onmouseenter={() => onBranchHover(packed.shown.label)}
            onmouseleave={onBranchLeave}
          >
            {#if isCurrentRef(packed.shown, current)}
              <svg class="mark" viewBox="0 0 16 16" aria-label="Estás aquí"><path d="M3.2 8.4 6.4 11.6 12.8 4.4" /></svg>
            {/if}
            <span class="name">{packed.shown.display ?? packed.shown.label}</span>
            <span class="glyphs">
              {@render kindIcon(packed.shown.kind === "head" ? "local" : packed.shown.kind)}
              {#if packed.shown.synced}{@render kindIcon("remote")}{/if}
            </span>
          </svelte:element>
        {/if}
        {#if packed.extra.length}
          <button type="button" class="more" title="Ver etiquetas" onclick={(event) => openRefs(event, refChips(commit.refs))} oncontextmenu={(event) => { event.preventDefault(); onmenu?.(event, commit); }}>+{packed.extra.length}</button>
        {/if}
      </div>
    {/each}
  </div>
  {/if}
  {#if fields.includes("graph")}
  {#if preferences?.showTreeInGraph}
  <div class="tree-view">
    {#each rows as commit, index}
      <div class="tree-row" style:padding-left="{index * 16 + 8}px">
        <span class="tree-branch">{commit.summary}</span>
      </div>
    {/each}
  </div>
  {:else}
  <svg {width} height={height} role="img" aria-label="Grafo de commits">
    {#if rows.findIndex((commit) => commit.id === selected) >= 0}
      <rect x="0" y={rows.findIndex((commit) => commit.id === selected) * row} {width} height={row} fill="#2a1c0c" />
    {/if}
    {#each rows as commit, index}
      <rect x="0" y={index * row} {width} height={row} fill={commit.pending ? "#f5b942" : tone(commit.lane)} opacity={commit.pending ? "0.07" : "0.05"} />
    {/each}
    {#each rows as commit, index}
      {#each commit.parent_lanes as parentLane, parentIndex}
        {@const parent = rows.findIndex((item) => item.id === commit.parents[parentIndex])}
        {#if laneFits(commit.lane, fit) && laneFits(parentLane, fit)}
        <path
          d={edge(x(commit.lane), y(index), x(parentLane), parent >= 0 ? y(parent) : y(index) + row)}
          stroke={commit.stash ? "#f5b942" : tone(parentLane)}
          stroke-width="3"
          stroke-linecap="round"
          stroke-dasharray={commit.stash ? "8 5" : undefined}
          fill="none"
          opacity="1"
        />
        {/if}
      {/each}
    {/each}
    {#each rows as commit, index}
      {#if laneFits(commit.lane, fit)}
      <g
        role="button"
        tabindex="0"
        onclick={() => { if (!commit.stash && (!commit.pending || commit.pendingCurrent)) onselect(commit); }}
        oncontextmenu={(event) => { event.preventDefault(); if (!commit.pending && !commit.stash) onmenu?.(event, commit); }}
        onkeydown={(event) => event.key === "Enter" && !commit.stash && (!commit.pending || commit.pendingCurrent) && onselect(commit)}
      >
        {#if selected === commit.id}
          <circle cx={x(commit.lane)} cy={y(index)} r="10" fill={commit.pending ? "#f5b942" : tone(commit.lane)} opacity="0.25" />
        {/if}
        {#if commit.pending}
          <rect x={x(commit.lane) - 6} y={y(index) - 6} width="12" height="12" rx="2.5" fill="#f5b942" stroke={commit.pendingCurrent && pendingConflicts ? "#e85d4c" : "#0a0908"} stroke-width="2" />
        {:else if commit.stash}
          <circle cx={x(commit.lane)} cy={y(index)} r="6" fill="#14110d" stroke="#f5b942" stroke-width="2.5" stroke-dasharray="3.5 2.5" />
        {:else}
          <circle cx={x(commit.lane)} cy={y(index)} r={selected === commit.id ? 8 : 6} fill={tone(commit.lane)} stroke="#0a0908" stroke-width="2" />
        {/if}
        {#if preferences?.showGhostBranch && hoverBranch && commit.refs?.some((ref) => ref.label === hoverBranch)}
          <text x={x(commit.lane) + 15} y={y(index) - 8} font-size="10" fill={groupColors[preferences?.defaultGroupColor] || "#8c7b66"}>{hoverBranch}</text>
        {/if}
        {#if preferences?.showDescriptionInGraph === "always" && commit.description}
          <text x={x(commit.lane) + 15} y={y(index) - 8} font-size="10" fill="#8c7b66">{commit.description}</text>
        {/if}
        <title>{preferences?.showDescriptionInGraph === "always" ? `${commit.summary}\n\n${commit.description || ""}` : commit.summary}</title>
      </g>
      {/if}
    {/each}
  </svg>
  {/if}
  {/if}
  {#each ["message", "author", "date", "hash"].filter((id) => fields.includes(id) && (preferences ? (id === "message" ? preferences.showMessageInGraph : id === "author" ? preferences.showAuthorInGraph : id === "date" ? preferences.showDateInGraph : true) : true)) as id (id)}
    <div class="rows">
      {#each rows as commit (commit.id)}
        <button
          type="button"
          class="commit-row {id}"
          class:selected={selected === commit.id}
          class:pending={commit.pending}
          class:stash={commit.stash}
          class:highlighted={isRowHighlighted(commit)}
          style:--lane-color={commit.pending || commit.stash ? "#f5b942" : tone(commit.lane)}
          draggable={id === "message" && !commit.pending && !commit.stash}
          ondragstart={(event) => event.dataTransfer?.setData("text/plain", `commit:${commit.id}`)}
          onclick={() => { if (!commit.stash && (!commit.pending || commit.pendingCurrent)) onselect(commit); }}
          oncontextmenu={(event) => { event.preventDefault(); if (!commit.pending && !commit.stash) onmenu?.(event, commit); }}
        >
          {#if id === "message"}<span class="summary">{commit.summary}</span>
          {:else if id === "author"}<span class="who">{preferences?.showAuthorInitials ? commit.author?.split(" ").map((n) => n[0]).join("") : commit.author}</span>
          {:else if id === "date"}<span class="ago">{ago(commit.time)}</span>
          {:else}<span class="hash">{commit.pending || commit.stash || !preferences?.showShaInGraph ? "" : commit.id.slice(0, 7)}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/each}
  </div>
</div>

{#snippet kindIcon(kind)}
  <svg viewBox="0 0 16 16" aria-hidden="true">
    {#if kind === "local"}
      <circle cx="4" cy="3" r="1.3" /><circle cx="4" cy="13" r="1.3" /><circle cx="11" cy="13" r="1.3" />
      <path d="M4 4.3v7.4M4 5.2h4.2a2.4 2.4 0 0 1 2.4 2.4V11.7" />
    {:else if kind === "remote"}
      <circle cx="8" cy="8" r="5.2" />
      <path d="M2.8 8h10.4M8 2.8c1.7 1.7 2.6 3.4 2.6 5.2s-.9 3.5-2.6 5.2c-1.7-1.7-2.6-3.4-2.6-5.2s.9-3.5 2.6-5.2" />
    {:else if kind === "tag"}
      <path d="M2.6 8.4 7.6 3.2h5v5L7.8 13a1.1 1.1 0 0 1-1.6 0L2.6 9.4a.8.8 0 0 1 0-1z" />
      <circle cx="10.4" cy="5.6" r="0.7" />
    {:else}
      <circle cx="8" cy="8" r="2" /><circle cx="8" cy="8" r="5.2" />
    {/if}
  </svg>
{/snippet}

{#if refMenu}
  <div class="ref-back" role="presentation" onclick={() => (refMenu = null)} onkeydown={(event) => { if (event.key === "Escape") refMenu = null; }}>
    <div class="ref-menu" style:left="{refMenu.x}px" style:top="{refMenu.y}px" role="menu" tabindex="-1" onclick={(event) => event.stopPropagation()} onkeydown={(event) => { if (event.key === "Escape") refMenu = null; }}>
      {#each refMenu.chips as chip (chip.kind + chip.label)}
        <button type="button" role="menuitem" aria-label="{chipKind(chip.kind)} {chip.label}" disabled={!refTarget(chip, branches)} onclick={() => chooseRef(chip)}>
          <span class="kind {chip.kind}" title={chipKind(chip.kind)}>{@render kindIcon(chip.kind)}</span>
          <span class="ref-name">{chip.display ?? chip.label}</span>
          {#if chip.synced}
            <span class="kind remote sync" title="Igual que origin/{chip.label}">{@render kindIcon("remote")}</span>
          {/if}
          {#if isCurrentRef(chip, current)}
            <svg class="tick" viewBox="0 0 16 16" aria-label="Estás aquí"><path d="M3.2 8.4 6.4 11.6 12.8 4.4" /></svg>
          {/if}
        </button>
      {/each}
    </div>
  </div>
{/if}

<style>
  .graph { display: flex; flex-direction: column; width: 100%; min-width: 0; background: #060504; }
  .colbar, .body { display: grid; grid-template-columns: var(--cols); align-items: start; }
  .colbar {
    position: sticky;
    top: 0;
    z-index: 4;
    background: #0c0a08;
    border-bottom: 1px solid #2a2218;
  }
  .layout.edge {
    position: absolute;
    top: 6px;
    right: 10px;
    z-index: 5;
  }
  .colhead {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 12px;
    color: #8c7b66;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    border-left: 1px solid #241c14;
  }
  .colhead:first-child { border-left: none; }
  .colhead:last-of-type { padding-right: 42px; }
  .colhead span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .body > svg { display: block; overflow: hidden; }
  .layout {
    width: 24px;
    height: 24px;
    padding: 0;
    border-radius: 8px;
    border-color: transparent;
    background: transparent;
    color: #8c7b66;
  }
  .layout:hover { background: #1a140e; border-color: transparent; color: #ffe7a3; }
  .layout.on {
    color: #ffe7a3;
    background: #1a140e;
    border-color: #3d3226;
    box-shadow: inset 0 0 0 1px #f5b94255;
  }
  .layout svg { width: 14px; height: 14px; display: block; margin: 0 auto; }
  .layout svg rect, .layout svg path { fill: none; stroke: currentColor; stroke-width: 1.6; }
  .grip {
    position: absolute;
    top: 0;
    right: -3px;
    width: 8px;
    height: 34px;
    padding: 0;
    border: none;
    border-radius: 0;
    background: transparent;
    cursor: col-resize;
    z-index: 2;
  }
  .grip::after {
    content: "";
    position: absolute;
    top: 10px;
    bottom: 10px;
    left: 3px;
    width: 2px;
    border-radius: 2px;
    background: #3d3226;
  }
  .grip:hover::after, .grip:active::after { background: #f5b942; }
  .body { align-items: start; }
  .refcol { min-width: 0; }
  .refcell {
    height: 36px;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 6px;
    padding: 0 12px;
    overflow: hidden;
    min-width: 0;
    background: color-mix(in srgb, var(--lane-color, transparent) 8%, transparent);
  }
  .label, .label:hover {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    min-width: 0;
    height: auto;
    min-height: 0;
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    box-shadow: none;
    background: transparent;
    color: var(--lane-color, #e7e2d8);
    font-size: 12.5px;
    font-weight: 650;
  }
  .label .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .label.tag { color: #b7d36a; }
  .label.pending {
    color: #ffe7a3;
    font-style: italic;
  }
  .pending-dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #f5b942;
    box-shadow: 0 0 6px #f5b942;
  }
  .commit-row.pending .summary { color: #ffe7a3; font-style: italic; }
  .label.pending.alert .pending-dot { background: #e85d4c; box-shadow: 0 0 6px #e85d4c; }
  .label.stash { color: #9fd8d0; font-style: italic; }
  .stash-dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1.6px dashed #2ec8c0;
  }
  .commit-row.stash .summary { color: #9fd8d0; font-style: italic; }
  .glyphs { display: inline-flex; align-items: center; gap: 3px; flex: none; color: color-mix(in srgb, var(--lane-color, #9a8b78) 65%, transparent); }
  .glyphs svg, .mark { width: 13px; height: 13px; display: block; }
  .glyphs svg path, .glyphs svg circle, .mark path {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .mark { flex: none; color: #7dcfb6; }
  .more {
    flex: none;
    height: 18px;
    padding: 0 6px;
    border-radius: 999px;
    border: 1px solid #6a5030;
    background: #1a140e;
    color: #ffe7a3;
    font-family: "Fragment Mono", monospace;
    font-size: 10px;
  }
  .more:hover { border-color: #f5b942; }
  .ref-back { position: fixed; inset: 0; z-index: 40; }
  .ref-menu {
    position: fixed;
    z-index: 41;
    min-width: 240px;
    max-width: 320px;
    padding: 6px;
    background: linear-gradient(180deg, #2c2418, #1c1712);
    border: 1px solid #6a5030;
    border-radius: 14px;
    box-shadow: 0 18px 40px #00000066;
  }
  .ref-menu button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    background: transparent;
    border-color: transparent;
    border-radius: 8px;
    padding: 6px 8px;
  }
  .ref-menu button:hover:not(:disabled) { background: #3a2a14; }
  .ref-menu button:disabled { opacity: 0.55; cursor: default; }
  .kind {
    flex: none;
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: 7px;
    border: 1px solid transparent;
  }
  .kind svg { width: 13px; height: 13px; }
  .kind svg path, .kind svg circle { fill: none; stroke: currentColor; stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; }
  .kind.local { color: #1a1208; background: linear-gradient(180deg, #ffe7a3, #f5b942); }
  .kind.remote { color: #e8c56b; background: #14110d; border-color: #5c4630; }
  .kind.tag { color: #d5e39a; background: #1a2214; border-color: #4a5a30; }
  .kind.head { color: #ffe7a3; background: #1a1208; border-color: #f5b942; }
  .kind.sync, .tick { margin-left: auto; }
  .kind.sync + .tick { margin-left: 0; }
  .tick { flex: none; width: 14px; height: 14px; color: #f5b942; }
  .tick path { fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
  .ref-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: "Fragment Mono", monospace;
    font-size: 12px;
  }
  .body > svg { display: block; }
  .rows { min-width: 0; }
  .commit-row, .label, .more, .layout {
    box-sizing: border-box;
    line-height: 1;
  }
  .commit-row:active, .label:active, .more:active, .layout:active { transform: none; }
  .commit-row, .commit-row:hover {
    height: 36px;
    min-height: 36px;
    max-height: 36px;
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 16px;
    width: 100%;
    margin: 0;
    padding: 0 12px;
    border: 0;
    border-radius: 0;
    box-shadow: none;
    background: color-mix(in srgb, var(--lane-color, transparent) 8%, transparent);
    text-align: left;
    color: #ffffff;
  }
  .commit-row:hover {
    background: color-mix(in srgb, var(--lane-color, transparent) 12%, transparent);
  }
  .commit-row.selected {
    background: color-mix(in srgb, var(--lane-color, transparent) 15%, transparent);
  }
  .summary {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13.5px;
    font-weight: 650;
    letter-spacing: -0.01em;
    color: #ffffff;
  }
  .ago {
    flex: none;
    color: #ffffff;
    font-size: 12px;
    font-weight: 500;
  }
  .meta {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
    max-width: 100%;
    color: #8d7b68;
    font-size: 11px;
  }
  .refs {
    display: flex;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
  }
  .ref {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    border-radius: 999px;
    padding: 1px 7px;
    font-family: "Fragment Mono", monospace;
    font-size: 10px;
    line-height: 16px;
    border: 1px solid transparent;
  }
  .ref.local {
    color: #1a1208;
    background: linear-gradient(180deg, #ffe7a3, #f5b942);
  }
  .ref.remote {
    color: #e8c56b;
    background: #14110d;
    border-color: #5c4630;
  }
  .ref.tag {
    color: #d5e39a;
    background: #1a2214;
    border-color: #4a5a30;
  }
  .ref.head {
    color: #ffe7a3;
    background: #1a1208;
    border-color: #f5b942;
  }
  .ref.head::before {
    content: "";
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #f5b942;
    box-shadow: 0 0 6px #f5b942;
    flex: none;
  }
  .who, .when { flex: none; white-space: nowrap; }
  .who { max-width: 120px; overflow: hidden; text-overflow: ellipsis; }
  .hash {
    margin-left: auto;
    flex: none;
    font-family: "Fragment Mono", monospace;
    color: #ffffff;
    letter-spacing: 0.02em;
  }
  .commit-row.author, .commit-row.date, .commit-row.hash { color: #ffffff; font-size: 12px; font-weight: 500; }
  .commit-row.hash { font-family: "Fragment Mono", monospace; letter-spacing: 0.02em; }
  .commit-row:hover, .refcell:hover { background: #14110d; }
  .commit-row.selected, .refcell.selected { background: #2a1c0c; }
  .commit-row.selected .summary, .commit-row.selected { color: #ffffff; }
  .commit-row.highlighted {
    background: color-mix(in srgb, var(--lane-color, #f5b942) 20%, transparent);
    box-shadow: inset 2px 0 0 var(--lane-color, #f5b942);
  }
  .tree-view {
    flex: 1;
    overflow: auto;
    padding: 12px;
    background: #100e0b;
  }
  .tree-row {
    padding: 4px 0;
    color: #f4ede0;
    font-size: 13px;
    border-left: 2px solid #2a2218;
    margin-left: 8px;
  }
  .tree-branch {
    margin-left: 8px;
  }
</style>
