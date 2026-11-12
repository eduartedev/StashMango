<script>
  let { commits = [], selected = "", onselect } = $props();
  const row = 36;
  const gap = 18;
  const tones = ["#F5B942", "#E8941A", "#FFE7A3", "#C47A12", "#F0C36A", "#A85A10"];

  function x(lane) {
    return 20 + lane * gap;
  }

  function tone(lane) {
    return tones[lane % tones.length];
  }

  function when(time) {
    if (!time) return "";
    return new Date(time * 1000).toLocaleDateString("es", { day: "2-digit", month: "short" });
  }

  let width = $derived(Math.max(56, (Math.max(0, ...commits.map((c) => c.lane)) + 2) * gap));
  let height = $derived(Math.max(row, commits.length * row));
</script>

<div class="graph">
  <svg {width} height={height} role="img" aria-label="Grafo de commits">
    {#each commits as commit, index}
      {#each commit.parent_lanes as parentLane, parentIndex}
        {@const parent = commits.findIndex((item) => item.id === commit.parents[parentIndex])}
        <line
          x1={x(commit.lane)}
          y1={index * row + 18}
          x2={x(parentLane)}
          y2={parent >= 0 ? parent * row + 18 : (index + 1) * row}
          stroke={tone(parentLane)}
          stroke-width="2"
          stroke-linecap="round"
          opacity="0.85"
        />
      {/each}
    {/each}
    {#each commits as commit, index}
      <circle
        cx={x(commit.lane)}
        cy={index * row + 18}
        r={selected === commit.id ? 7 : 5.5}
        fill={selected === commit.id ? "#ffe7a3" : tone(commit.lane)}
        stroke="#1a1208"
        stroke-width="1"
        role="button"
        tabindex="0"
        onclick={() => onselect(commit)}
        onkeydown={(event) => event.key === "Enter" && onselect(commit)}
      >
        <title>{commit.summary}</title>
      </circle>
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
        <span class="ref" class:empty={!commit.refs}>{commit.refs}</span>
        <span class="muted">{commit.author}</span>
        <span class="muted">{when(commit.time)}</span>
        <span class="hash">{commit.id.slice(0, 7)}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .graph { display: grid; grid-template-columns: auto minmax(0, 1fr); align-items: start; }
  svg { display: block; }
  .rows { min-width: 0; }
  .commit-row {
    height: 36px;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto auto auto;
    gap: 10px;
    align-items: center;
    width: 100%;
    margin: 0;
    padding: 0 12px;
    border: none;
    border-radius: 10px;
    background: transparent;
    text-align: left;
  }
  .summary { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ref {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #1a1208;
    background: #f5b942;
    border-radius: 999px;
    padding: 1px 8px;
    font-size: 11px;
  }
  .ref.empty { visibility: hidden; padding: 0; }
  .hash { font-family: "Fragment Mono", monospace; font-size: 12px; color: #ffe7a3; }
  .commit-row.selected { background: #2c2418; }
  .muted { color: #b7a48a; font-size: 13px; }
</style>
