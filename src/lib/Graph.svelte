<script>
  let { commits = [], selected = "", onselect } = $props();
  const row = 52;
  const gap = 26;
  const tones = [
    "#F5B942",
    "#E07A3D",
    "#E8C56B",
    "#D45D4A",
    "#C6D36A",
    "#3EAA8C",
    "#6EB0D6",
    "#E08BB0",
    "#C9A0E0",
    "#F0A05A",
    "#7DCFB6",
    "#E6D3A1",
  ];

  function x(lane) {
    return 22 + lane * gap;
  }

  function y(index) {
    return index * row + row / 2;
  }

  function tone(lane) {
    return tones[lane % tones.length];
  }

  function when(time) {
    if (!time) return "";
    return new Date(time * 1000).toLocaleDateString("es", { day: "2-digit", month: "short" });
  }

  function edge(x1, y1, x2, y2) {
    if (Math.abs(x1 - x2) < 1) return `M ${x1} ${y1} V ${y2}`;
    const drop = Math.min(20, Math.abs(y2 - y1) / 2);
    const yBend = y2 - Math.sign(y2 - y1) * drop;
    return `M ${x1} ${y1} V ${yBend} C ${x1} ${y2}, ${x2} ${yBend}, ${x2} ${y2}`;
  }

  let width = $derived(Math.max(64, (Math.max(0, ...commits.map((c) => c.lane)) + 1) * gap + 28));
  let height = $derived(Math.max(row, commits.length * row));
</script>

<div class="graph">
  <svg {width} height={height} role="img" aria-label="Grafo de commits">
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
        <path
          d={edge(x(commit.lane), y(index), x(parentLane), parent >= 0 ? y(parent) : y(index) + row)}
          stroke={tone(parentLane)}
          stroke-width="2.5"
          stroke-linecap="round"
          fill="none"
          opacity="0.9"
        />
      {/each}
    {/each}
    {#each commits as commit, index}
      <g
        role="button"
        tabindex="0"
        filter={selected === commit.id ? "url(#glow)" : undefined}
        onclick={() => onselect(commit)}
        onkeydown={(event) => event.key === "Enter" && onselect(commit)}
      >
        <circle cx={x(commit.lane)} cy={y(index)} r={selected === commit.id ? 8 : 6.5} fill="#140f0b" stroke={tone(commit.lane)} stroke-width="2.5" />
        <circle cx={x(commit.lane)} cy={y(index)} r={selected === commit.id ? 3.5 : 2.6} fill={selected === commit.id ? "#ffe7a3" : tone(commit.lane)} />
        <title>{commit.summary}</title>
      </g>
    {/each}
  </svg>

  <div class="rows">
    {#each commits as commit}
      <button
        class="commit-row"
        class:selected={selected === commit.id}
        draggable="true"
        ondragstart={(event) => event.dataTransfer?.setData("text/plain", `commit:${commit.id}`)}
        onclick={() => onselect(commit)}
      >
        <span class="summary">{commit.summary}</span>
        <span class="meta">
          {#if commit.refs}<span class="ref">{commit.refs}</span>{/if}
          <span>{commit.author}</span>
          <span>{when(commit.time)}</span>
          <span class="hash">{commit.id.slice(0, 7)}</span>
        </span>
      </button>
    {/each}
  </div>
</div>

<style>
  .graph { display: grid; grid-template-columns: auto minmax(0, 1fr); align-items: start; min-width: max-content; }
  svg { display: block; }
  .rows { min-width: 280px; }
  .commit-row {
    height: 52px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    margin: 0;
    padding: 0 14px;
    border: none;
    border-radius: 12px;
    background: transparent;
    text-align: left;
  }
  .summary {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 14px;
  }
  .meta {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
    max-width: 100%;
    color: #b7a48a;
    font-size: 12px;
  }
  .ref {
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #1a1208;
    background: #f5b942;
    border-radius: 999px;
    padding: 1px 8px;
    font-size: 11px;
  }
  .hash { font-family: "Fragment Mono", monospace; color: #ffe7a3; }
  .commit-row:hover { background: #14110d; }
  .commit-row.selected { background: #1a140e; box-shadow: inset 3px 0 0 #f5b942; }
</style>
