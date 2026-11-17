<script>
  import { clampColumn, columnTemplate } from "$lib/columns.js";
  import { graphFit, laneFits } from "$lib/graphFit.js";
  import { menuPosition } from "$lib/branchMenu.js";
  import { chipKind, isCurrentRef, packRefs, refChips, refTarget } from "$lib/refChips.js";

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
    fields = ["refs", "graph", "message", "date"],
  } = $props();
  let refMenu = $state(null);
  const row = 36;
  const tones = ["#2ec8c0", "#e14b8c", "#8fd14f", "#f0a05a", "#6eb0d6", "#c9a0e0", "#f5b942", "#3eaa8c", "#e07a3d", "#7dcfb6"];

  let laneCount = $derived(commits.length ? Math.max(...commits.map((commit) => commit.lane)) + 1 : 0);
  let fit = $derived(graphFit(laneCount, graphWidth));

  function x(lane) {
    return 16 + lane * fit.gap;
  }

  function y(index) {
    return index * row + row / 2;
  }

  function tone(lane) {
    return tones[lane % tones.length];
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
    const drop = Math.min(20, Math.abs(y2 - y1) / 2);
    const yBend = y2 - Math.sign(y2 - y1) * drop;
    return `M ${x1} ${y1} V ${yBend} C ${x1} ${y2}, ${x2} ${yBend}, ${x2} ${y2}`;
  }

  let width = $derived(fit.width);
  let cols = $derived(columnTemplate(fields, refsWidth, width));
  const heads = { refs: "Ramas", graph: "Grafo", message: "Mensaje", author: "Autor", date: "Fecha", hash: "Hash" };
  let height = $derived(Math.max(row, commits.length * row));

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
    {#each commits as commit}
      {@const packed = packRefs(refChips(commit.refs))}
      <div class="refcell" class:selected={selected === commit.id} role="presentation">
        {#if packed.shown}
          <svelte:element
            this={packed.extra.length ? "button" : "span"}
            type={packed.extra.length ? "button" : undefined}
            class="label {packed.shown.kind}"
            title={packed.shown.synced ? `${packed.shown.title} · origin/${packed.shown.label}` : packed.shown.title}
            onclick={packed.extra.length ? (event) => openRefs(event, refChips(commit.refs)) : undefined}
          >
            {#if isCurrentRef(packed.shown, current)}
              <svg class="mark" viewBox="0 0 16 16" aria-label="Estás aquí"><path d="M3.2 8.4 6.4 11.6 12.8 4.4" /></svg>
            {/if}
            <span class="name">{packed.shown.label}</span>
            <span class="glyphs">
              {@render kindIcon(packed.shown.kind === "head" ? "local" : packed.shown.kind)}
              {#if packed.shown.synced}{@render kindIcon("remote")}{/if}
            </span>
          </svelte:element>
        {/if}
        {#if packed.extra.length}
          <button type="button" class="more" title="Ver etiquetas" onclick={(event) => openRefs(event, refChips(commit.refs))}>+{packed.extra.length}</button>
        {/if}
      </div>
    {/each}
  </div>
  {/if}
  {#if fields.includes("graph")}
  <svg {width} height={height} role="img" aria-label="Grafo de commits">
    {#if commits.findIndex((commit) => commit.id === selected) >= 0}
      <rect x="0" y={commits.findIndex((commit) => commit.id === selected) * row} {width} height={row} fill="#2a1c0c" />
    {/if}
    <defs>
      <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
        <feGaussianBlur stdDeviation="2.2" result="blur" />
        <feMerge>
          <feMergeNode in="blur" />
          <feMergeNode in="SourceGraphic" />
        </feMerge>
      </filter>
    </defs>
    {#each commits as commit, index}
      {#each commit.parent_lanes as parentLane, parentIndex}
        {@const parent = commits.findIndex((item) => item.id === commit.parents[parentIndex])}
        {#if laneFits(commit.lane, fit) && laneFits(parentLane, fit)}
        <path
          d={edge(x(commit.lane), y(index), x(parentLane), parent >= 0 ? y(parent) : y(index) + row)}
          stroke={tone(parentLane)}
          stroke-width="2"
          stroke-linecap="round"
          fill="none"
          opacity="0.9"
        />
        {/if}
      {/each}
    {/each}
    {#each commits as commit, index}
      {#if laneFits(commit.lane, fit)}
      <g
        role="button"
        tabindex="0"
        filter={selected === commit.id ? "url(#glow)" : undefined}
        onclick={() => onselect(commit)}
        onkeydown={(event) => event.key === "Enter" && onselect(commit)}
      >
        <circle cx={x(commit.lane)} cy={y(index)} r={selected === commit.id ? 7 : 5.5} fill="#100e0b" stroke={tone(commit.lane)} stroke-width="2" />
        <circle cx={x(commit.lane)} cy={y(index)} r="2" fill={tone(commit.lane)} />
        <title>{commit.summary}</title>
      </g>
      {/if}
    {/each}
  </svg>
  {/if}
  {#each ["message", "author", "date", "hash"].filter((id) => fields.includes(id)) as id (id)}
    <div class="rows">
      {#each commits as commit (commit.id)}
        <button
          type="button"
          class="commit-row {id}"
          class:selected={selected === commit.id}
          draggable={id === "message"}
          ondragstart={(event) => event.dataTransfer?.setData("text/plain", `commit:${commit.id}`)}
          onclick={() => onselect(commit)}
        >
          {#if id === "message"}<span class="summary">{commit.summary}</span>
          {:else if id === "author"}<span class="who">{commit.author}</span>
          {:else if id === "date"}<span class="ago">{ago(commit.time)}</span>
          {:else}<span class="hash">{commit.id.slice(0, 7)}</span>
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
          <span class="ref-name">{chip.label}</span>
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
    justify-content: flex-end;
    gap: 4px;
    padding: 0 10px 0 8px;
    overflow: hidden;
    min-width: 0;
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
    color: #e7e2d8;
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
  .label.remote { color: #d7c4a4; }
  .glyphs { display: inline-flex; align-items: center; gap: 3px; flex: none; color: #9a8b78; }
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
    background: transparent;
    text-align: left;
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
    color: #f4f1ea;
  }
  .ago {
    flex: none;
    color: #8d8274;
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
    color: #c4b09a;
    letter-spacing: 0.02em;
  }
  .commit-row.author, .commit-row.date, .commit-row.hash { color: #8c7b66; font-size: 12px; font-weight: 500; }
  .commit-row.hash { font-family: "Fragment Mono", monospace; letter-spacing: 0.02em; }
  .commit-row:hover, .refcell:hover { background: #14110d; }
  .commit-row.selected, .refcell.selected { background: #2a1c0c; }
  .commit-row.selected .summary, .commit-row.selected { color: #f4ede0; }
</style>
