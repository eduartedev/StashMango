<script>
  import { open } from "@tauri-apps/plugin-dialog";
  import { call } from "$lib/api";
  import Graph from "$lib/Graph.svelte";
  import Merge from "$lib/Merge.svelte";

  let screen = $state("home");
  let error = $state("");
  let notice = $state("");
  let recents = $state([]);
  let tabs = $state([]);
  let activePath = $state("");
  let drafts = $state({});
  let phrases = $state({});
  let info = $state(null);
  let commits = $state([]);
  let openLanes = $state([]);
  let done = $state(false);
  let branches = $state([]);
  let worktrees = $state([]);
  let stashes = $state([]);
  let prs = $state([]);
  let selected = $state(null);
  let files = $state([]);
  let diff = $state(null);
  let message = $state("");
  let profile = $state(null);
  let profiles = $state({ profiles: [], repos: [] });
  let conflict = $state(null);
  let remote = $state("");
  let passphrase = $state("");
  let newBranch = $state("");
  let prTitle = $state("");
  let prHead = $state("");
  let prBase = $state("main");
  let modal = $state(null);
  let dragMenu = $state(null);
  let steps = $state([]);
  const services = [
    { label: "GitHub", host: "github.com" },
    { label: "GitLab", host: "gitlab.com" },
    { label: "Bitbucket", host: "bitbucket.org" },
  ];
  let profileForm = $state({ id: "default", label: "default", name: "", email: "" });
  let serviceTokens = $state({ "github.com": "", "gitlab.com": "", "bitbucket.org": "" });
  let serviceSaved = $state({ "github.com": false, "gitlab.com": false, "bitbucket.org": false });

  async function boot() {
    try {
      recents = await call("recents");
    } catch (err) {
      const text = String(err);
      if (!text.includes("invoke")) error = text;
    }
  }
  boot();

  async function fail(run) {
    error = "";
    try {
      return await run();
    } catch (err) {
      const text = String(err);
      if (!text.includes("invoke")) error = text;
    }
  }

  async function loadRepo() {
    info = await call("refresh");
    branches = await call("branches");
    worktrees = await call("worktrees");
    stashes = await call("stash_list");
    profile = await call("active_profile");
    remote = await call("remote");
    profiles = await call("profiles");
    commits = [];
    openLanes = [];
    done = false;
    await more();
    touchTab();
  }

  function repoName(path) {
    const parts = path.split(/[\\/]/).filter(Boolean);
    return parts[parts.length - 1] || path;
  }

  function touchTab() {
    if (!info) return;
    const tab = { path: info.path, name: repoName(info.path), branch: info.branch };
    const index = tabs.findIndex((item) => item.path === info.path);
    tabs = index >= 0 ? tabs.map((item, i) => (i === index ? tab : item)) : [...tabs, tab];
    activePath = info.path;
  }

  async function choose(path) {
    await fail(async () => {
      if (activePath) {
        drafts = { ...drafts, [activePath]: message };
        phrases = { ...phrases, [activePath]: passphrase };
      }
      const opened = await call("open_repo", { path });
      const same = opened.path === activePath;
      info = opened;
      screen = "repo";
      touchTab();
      if (!same) {
        selected = null;
        files = [];
        diff = null;
        conflict = null;
        steps = [];
        prs = [];
        commits = [];
        dragMenu = null;
        message = drafts[info.path] || "";
        passphrase = phrases[info.path] || "";
      }
      await loadRepo();
    });
  }

  async function focusTab(path) {
    if (path === activePath && screen === "repo") return;
    await choose(path);
  }

  async function closeTab(path) {
    const remaining = tabs.filter((tab) => tab.path !== path);
    tabs = remaining;
    const nextDrafts = { ...drafts };
    delete nextDrafts[path];
    drafts = nextDrafts;
    const nextPhrases = { ...phrases };
    delete nextPhrases[path];
    phrases = nextPhrases;
    if (path !== activePath) return;
    const neighbor = remaining[remaining.length - 1];
    activePath = "";
    if (!neighbor) {
      info = null;
      screen = "home";
      return;
    }
    await choose(neighbor.path);
  }

  async function browse() {
    const path = await open({ directory: true });
    if (path) await choose(path);
  }

  async function more() {
    if (done) return;
    const page = await call("log_page", { skip: commits.length, openLanes });
    commits = [...commits, ...page.commits];
    openLanes = page.open_lanes;
    done = page.done;
  }

  function onScroll(event) {
    const el = event.currentTarget;
    if (el.scrollTop + el.clientHeight > el.scrollHeight - 40) more();
  }

  async function pickCommit(commit) {
    selected = commit;
    conflict = null;
    diff = null;
    files = await call("commit_files", { sha: commit.id });
  }

  async function showDiff(path, cached) {
    selected = null;
    conflict = null;
    diff = await call("diff_file", { path, cached });
    diff.cached = cached;
  }

  function changedLines(text) {
    const hunks = [];
    let current = null;
    for (const line of (text || "").split("\n")) {
      if (line.startsWith("@@")) {
        current = { header: line, lines: [] };
        hunks.push(current);
      } else if (current && (line.startsWith("+") || line.startsWith("-"))) {
        current.lines.push(line);
      }
    }
    return hunks;
  }

  async function stageHunk(index) {
    await fail(async () => {
      await call("apply_selection", { path: diff.path, cached: diff.cached, hunks: [index], lines: [] });
      await loadRepo();
      await showDiff(diff.path, diff.cached);
    });
  }

  async function stageLine(hunk, line) {
    await fail(async () => {
      await call("apply_selection", {
        path: diff.path,
        cached: diff.cached,
        hunks: [],
        lines: [{ hunk, line }],
      });
      await loadRepo();
      await showDiff(diff.path, false);
    });
  }

  function ask(text, action) {
    modal = { text, action };
  }

  async function dropOn(branch, data) {
    if (data.startsWith("branch:")) {
      const source = data.slice(7);
      dragMenu = { source, branch };
    } else if (data.startsWith("commit:")) {
      const sha = data.slice(7);
      ask(`Cherry-pick de ${sha.slice(0, 7)} en ${branch}.`, async () => {
        await call("switch_branch", { name: branch });
        await call("cherry_pick", { sha });
        await loadRepo();
      });
    }
  }

  function blankTokens() {
    serviceTokens = { "github.com": "", "gitlab.com": "", "bitbucket.org": "" };
  }

  async function editProfile(item) {
    profileForm = { id: item.id, label: item.label, name: item.name, email: item.email };
    blankTokens();
    const saved = { "github.com": false, "gitlab.com": false, "bitbucket.org": false };
    for (const service of services) {
      saved[service.host] = await call("has_token", { profileId: item.id, host: service.host });
    }
    serviceSaved = saved;
  }

  async function loadSettings() {
    profiles = await call("profiles");
    const current = profiles.profiles.find((item) => item.id === profileForm.id) || profiles.profiles[0];
    if (current) await editProfile(current);
  }

  function openSettings() {
    screen = screen === "settings" ? (info ? "repo" : "home") : "settings";
    if (screen === "settings") fail(loadSettings);
  }

  function startProfile() {
    profileForm = { id: crypto.randomUUID(), label: "", name: "", email: "" };
    blankTokens();
    serviceSaved = { "github.com": false, "gitlab.com": false, "bitbucket.org": false };
  }

  async function saveCurrent() {
    profiles = await call("save_profile", { profile: profileForm });
    for (const service of services) {
      const value = serviceTokens[service.host];
      if (!value) continue;
      await call("save_token", { profileId: profileForm.id, host: service.host, token: value });
      serviceSaved = { ...serviceSaved, [service.host]: true };
    }
    blankTokens();
    notice = "Perfil guardado.";
  }

  async function saveResult(text) {
    await fail(async () => {
      await call("write_result", { path: conflict.path, text });
      await call("mark_resolved", { path: conflict.path });
      conflict = null;
      await loadRepo();
    });
  }
</script>

{#snippet mango(size)}
  <svg class="mark" width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
    <path d="M32 58c-14 0-22-12-20-26 2-16 14-24 20-26 6 2 18 10 20 26 2 14-6 26-20 26z" fill="#F5B942"/>
    <path d="M32 54c-8 0-14-8-12-18 1-8 7-14 12-16 5 2 11 8 12 16 2 10-4 18-12 18z" fill="#E8941A"/>
    <path d="M32 8c1 7 6 11 12 12-7 1-11 5-12 11-1-6-5-10-12-11 6-1 11-5 12-12z" fill="#8EA34A"/>
  </svg>
{/snippet}

<div class="app">
  <header class="top">
    <button class="brand" onclick={() => (screen = "home")}>{@render mango(28)} StashMango</button>
    {#if screen === "repo" && info}
      <div class="dock">
        <span class="chip">{info.branch || "HEAD"}</span>
        <button onclick={() => fail(() => call("network", { action: "fetch", passphrase: passphrase || null }).then(loadRepo))}>Fetch</button>
        <button onclick={() => fail(() => call("network", { action: "pull", passphrase: passphrase || null }).then(loadRepo))}>Pull</button>
        <button class="primary" onclick={() => fail(() => call("network", { action: "push", passphrase: passphrase || null }).then(loadRepo))}>Push</button>
        <input class="secret" placeholder="Frase SSH" bind:value={passphrase} type="password" />
      </div>
    {/if}
    <span class="spacer"></span>
    <button class="ghost" onclick={openSettings}>Configuración</button>
  </header>
  {#if tabs.length}
    <div class="tabs">
      {#each tabs as tab (tab.path)}
        <div class="tab" class:active={tab.path === activePath && screen === "repo"}>
          <button class="tab-main" onclick={() => focusTab(tab.path)}>
            <span>{tab.name}</span>
            <small>{tab.branch || "HEAD"}</small>
          </button>
          <button class="x" aria-label="Cerrar {tab.name}" onclick={() => closeTab(tab.path)}>×</button>
        </div>
      {/each}
      <button class="tab-add" onclick={browse}>+</button>
    </div>
  {/if}

  {#if error}<p class="banner error">{error}</p>{/if}
  {#if notice}<p class="banner ok">{notice}</p>{/if}

  {#if screen === "home"}
    <section class="home">
      <div class="hero">
        {@render mango(92)}
        <div>
          <p class="kicker">Puesto de noche</p>
          <h1>Repos recientes</h1>
          <p class="lede">El grafo ocupa el centro. Las ramas, el diff y los pull requests responden a lo que tienes abierto.</p>
          <button class="primary" onclick={browse}>Abrir carpeta</button>
        </div>
      </div>
      {#if recents.length === 0}
        <div class="stage-empty">
          <svg viewBox="0 0 720 200" aria-hidden="true">
            <path d="M48 36v128" stroke="#E8941A" stroke-width="2"/>
            <path d="M48 36c40 0 40 48 80 48" stroke="#F5B942" stroke-width="2" fill="none"/>
            <path d="M128 84v80" stroke="#F5B942" stroke-width="2"/>
            <path d="M128 100c36 0 36 36 72 36" stroke="#FFE7A3" stroke-width="2" fill="none"/>
            <circle cx="48" cy="36" r="7" fill="#FFE7A3"/>
            <circle cx="48" cy="92" r="6" fill="#F5B942"/>
            <circle cx="48" cy="148" r="6" fill="#E8941A"/>
            <circle cx="128" cy="84" r="6" fill="#F5B942"/>
            <circle cx="128" cy="156" r="6" fill="#C47A12"/>
            <circle cx="200" cy="136" r="6" fill="#FFE7A3"/>
          </svg>
          <p>Todavía no hay repos. El primero queda en una ficha y el grafo se enciende aquí.</p>
        </div>
      {/if}
      <div class="cards">
        {#each recents as repo, index}
          <button class="card" style="animation-delay: {index * 40}ms" onclick={() => choose(repo.path)}>
            <strong>{repo.name}</strong>
            <span class="pill">{repo.branch || "HEAD"}</span>
            <small>{repo.path}</small>
          </button>
        {/each}
      </div>
    </section>
  {:else if screen === "settings"}
    <section class="home settings">
      <header class="settings-head">
        <p class="kicker">Firmas y acceso</p>
        <h1>Configuración</h1>
      </header>
      <article class="plate">
        <div class="plate-head">
          <h2>Perfiles</h2>
          <p>El perfil default ya existe. El nombre, el correo y los tokens de cada servicio se guardan en este perfil.</p>
        </div>
        {#if profiles.profiles.length}
          <ul class="people">
            {#each profiles.profiles as item}
              <li class:active={item.id === profileForm.id}>
                <button class="branch" onclick={() => fail(() => editProfile(item))}>
                  <strong>{item.label}</strong>
                  <span class="muted">{item.name ? `${item.name} <${item.email}>` : "Sin firma todavía"}</span>
                </button>
                {#if item.id !== "default"}
                  <button class="danger" onclick={() => fail(async () => { profiles = await call("delete_profile", { id: item.id }); if (profileForm.id === item.id) await editProfile(profiles.profiles.find((profile) => profile.id === "default")); })}>Borrar</button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
        <div class="fields">
          <label>Etiqueta <input bind:value={profileForm.label} /></label>
          <label>Nombre <input bind:value={profileForm.name} /></label>
          <label>Correo <input bind:value={profileForm.email} /></label>
        </div>
        <div class="services">
          <h3>Servicios</h3>
          {#each services as service}
            <div class="service">
              <div>
                <strong>{service.label}</strong>
                <span class="muted">{serviceSaved[service.host] ? "Token guardado" : "Sin token"}</span>
              </div>
              <input type="password" placeholder="Pegar token" bind:value={serviceTokens[service.host]} autocomplete="off" />
              {#if serviceSaved[service.host]}
                <button class="danger" onclick={() => fail(async () => { await call("clear_token", { profileId: profileForm.id, host: service.host }); serviceSaved = { ...serviceSaved, [service.host]: false }; })}>Quitar</button>
              {/if}
            </div>
          {/each}
        </div>
        <div class="plate-foot spread">
          <button onclick={startProfile}>Otro perfil</button>
          <button class="primary" onclick={() => fail(saveCurrent)}>Guardar</button>
        </div>
      </article>
      <article class="plate">
        <div class="plate-head">
          <h2>Git portable</h2>
          <p>Apagado de inicio. Solo agrega la carpeta de StashMango al final del PATH de tu usuario.</p>
        </div>
        <div class="plate-foot split">
          <button onclick={() => fail(() => call("set_external_git", { enabled: true }))}>Usar este Git fuera de StashMango</button>
          <button onclick={() => fail(() => call("set_external_git", { enabled: false }))}>Quitarlo del PATH</button>
        </div>
      </article>
    </section>
  {:else if info}
    <div class="work">
      <aside class="side">
        <div class="section">
          <h2>Ramas</h2>
          {#each branches as branch}
            <button
              class="branch"
              class:current={branch === info.branch}
              draggable="true"
              ondragstart={(event) => event.dataTransfer?.setData("text/plain", `branch:${branch}`)}
              ondragover={(event) => event.preventDefault()}
              ondrop={(event) => { event.preventDefault(); dropOn(branch, event.dataTransfer?.getData("text/plain") || ""); }}
              onclick={() => fail(async () => { await call("switch_branch", { name: branch }); await loadRepo(); })}
            >{branch}</button>
          {/each}
          <div class="rowish">
            <input placeholder="nueva rama" bind:value={newBranch} />
            <button onclick={() => fail(async () => { await call("create_branch", { name: newBranch }); newBranch = ""; await loadRepo(); })}>Crear</button>
          </div>
        </div>
        <div class="section">
          <h2>Worktrees</h2>
          {#each worktrees as tree}
            <button class="branch" onclick={() => choose(tree.path)}>{tree.branch || tree.head.slice(0, 7)}<small class="muted">{tree.path}</small></button>
            {#if tree.path !== info.path}
              <button class="danger" onclick={() => ask(`Quitar el worktree ${tree.path}. Si hay cambios, git se niega.`, async () => { await call("remove_worktree", { path: tree.path, confirm: true }); await loadRepo(); })}>Quitar</button>
            {/if}
          {/each}
          <button onclick={async () => { const path = await open({ directory: true }); if (path && newBranch) await fail(async () => { await call("add_worktree", { path, branch: newBranch }); await loadRepo(); }); }}>Añadir en carpeta</button>
        </div>
        <div class="section">
          <h2>Stash</h2>
          {#each stashes as stash}<p class="mono">{stash}</p>{/each}
          <button onclick={() => fail(async () => { await call("stash_push", { message: message || "stash" }); await loadRepo(); })}>Guardar</button>
          <button onclick={() => fail(async () => { await call("stash_pop"); await loadRepo(); })}>Pop</button>
          <button class="danger" onclick={() => ask("Soltar el stash más reciente.", async () => { await call("stash_drop", { confirm: true }); await loadRepo(); })}>Drop</button>
        </div>
        <div class="section">
          <h2>Pull requests</h2>
          <button onclick={() => fail(async () => { prs = await call("list_prs"); })}>Actualizar</button>
          {#each prs as pr}
            <p>{pr.title}<span class="muted"> {pr.head} → {pr.base} · {pr.author}</span></p>
            <button onclick={() => fail(async () => { diff = { path: pr.title, text: await call("pr_diff", { number: pr.number }), too_big: false, binary: false, cached: false, hunks: 0 }; })}>Diff</button>
            <button onclick={() => fail(() => call("merge_pr", { number: pr.number }))}>Merge</button>
          {/each}
          <input placeholder="título" bind:value={prTitle} />
          <input placeholder="head" bind:value={prHead} />
          <input placeholder="base" bind:value={prBase} />
          <button onclick={() => fail(() => call("create_pr", { title: prTitle, head: prHead, base: prBase }))}>Crear PR</button>
        </div>
      </aside>

      <main class="graph-wrap" onscroll={onScroll}>
        <Graph {commits} selected={selected?.id || ""} onselect={pickCommit} />
        {#if !done}<button onclick={more}>Más historial</button>{/if}
      </main>

      <aside class="detail">
        <div class="section">
          <h2>Trabajo</h2>
          <p class="muted">{profile?.name ? `${profile.name} <${profile.email}>` : "Identidad de git"}</p>
          <select onchange={(event) => fail(() => call("set_repo_profile", { profileId: event.target.value || null }).then(() => call("active_profile")).then((value) => profile = value))}>
            <option value="">Sin perfil</option>
            {#each profiles.profiles as item}
              <option value={item.id} selected={profile?.id === item.id}>{item.label}</option>
            {/each}
          </select>
          {#each info.files as file}
            <div class="rowish">
              <button class="file" class:conflict={file.conflict} onclick={() => file.conflict ? fail(async () => { conflict = await call("conflict_text", { path: file.path }); diff = null; }) : showDiff(file.path, false)}>{file.path}</button>
              {#if !file.conflict}
                <button onclick={() => fail(async () => { await call("stage_file", { path: file.path, stage: !file.staged }); await loadRepo(); })}>{file.staged ? "Unstage" : "Stage"}</button>
              {/if}
            </div>
          {/each}
          <textarea rows="4" placeholder="Mensaje del commit" bind:value={message}></textarea>
          <button class="primary" onclick={() => fail(async () => { await call("commit", { message }); message = ""; await loadRepo(); })}>Commit</button>
        </div>
        {#if diff}
          <div class="section">
            <h2>{diff.path}</h2>
            {#if diff.too_big || diff.binary}
              <p>No se carga el archivo{diff.too_big ? " porque pasa el tope" : " porque es binario"}.</p>
              <button onclick={() => fail(() => call("open_external", { path: diff.path }))}>Abrir afuera</button>
            {:else}
              {#each changedLines(diff.text) as hunk, hunkIndex}
                <button onclick={() => stageHunk(hunkIndex)}>Stage hunk</button>
                {#each hunk.lines as line, lineIndex}
                  <div class="diff-line" class:add={line.startsWith("+")} class:del={line.startsWith("-")}>
                    <button onclick={() => stageLine(hunkIndex, lineIndex)}>línea</button>
                    <span class="mono">{line}</span>
                  </div>
                {/each}
              {/each}
            {/if}
          </div>
        {/if}
        {#if conflict}
          <div class="section">
            {#if conflict.too_big}
              <p>El conflicto pasa el tope.</p>
              <button onclick={() => fail(() => call("open_external", { path: conflict.path }))}>Abrir afuera</button>
              <button onclick={() => fail(() => call("mergetool", { path: conflict.path }))}>Herramienta de merge</button>
            {:else}
              <Merge path={conflict.path} ours={conflict.ours} theirs={conflict.theirs} bind:result={conflict.result} onsave={saveResult} />
              <button onclick={() => ask("Quedarme con el nuestro pisa el archivo.", async () => { await call("resolve_side", { path: conflict.path, ours: true, confirm: true }); conflict = null; await loadRepo(); })}>Quedarme con el nuestro</button>
              <button onclick={() => ask("Quedarme con el suyo pisa el archivo.", async () => { await call("resolve_side", { path: conflict.path, ours: false, confirm: true }); conflict = null; await loadRepo(); })}>Quedarme con el suyo</button>
            {/if}
          </div>
        {/if}
        <div class="section">
          <h2>Rebase visual</h2>
          <input placeholder="base, por ejemplo main" bind:value={prBase} />
          <button onclick={() => (steps = commits.slice(0, 12).map((commit) => ({ sha: commit.id, action: "pick", message: commit.summary })))}>Cargar</button>
          {#each steps as step, index}
            <div class="rowish">
              <span class="mono">{step.sha.slice(0, 7)}</span>
              <select bind:value={step.action}>
                <option value="pick">pick</option>
                <option value="drop">drop</option>
                <option value="reword">reword</option>
              </select>
              <input bind:value={step.message} />
              <button onclick={() => { const next = [...steps]; const swap = next[index - 1]; if (!swap) return; next[index - 1] = next[index]; next[index] = swap; steps = next; }}>Subir</button>
            </div>
          {/each}
          <button onclick={() => ask("Esto reescribe la rama con la lista de arriba.", async () => { await call("replay", { base: prBase, steps: [...steps].reverse() }); steps = []; await loadRepo(); })}>Aplicar</button>
        </div>
        {#if selected}
          <div class="section">
            <h2>{selected.summary}</h2>
            <p class="muted">{selected.author} · {selected.refs}</p>
            {#each files as file}<p class="mono">{file}</p>{/each}
            <button class="danger" onclick={() => ask(`Reset hard a ${selected.id.slice(0, 7)}. El trabajo sin commit se pierde. Puedes volver al commit anterior en esta sesión.`, async () => { await call("reset_hard", { target: selected.id, confirm: true }); await loadRepo(); })}>Reset hard</button>
            <button onclick={() => fail(async () => { await call("undo_reset"); await loadRepo(); })}>Volver a ese commit</button>
          </div>
        {/if}
      </aside>
    </div>
  {/if}

  {#if dragMenu}
    <div class="modal-back">
      <div class="modal">
        <p class="kicker">Soltar rama</p>
        <h2>Merge de {dragMenu.source} en {dragMenu.branch}</h2>
        <p>Rebase de {dragMenu.source} sobre {dragMenu.branch}</p>
        <div class="actions">
          <button onclick={() => (dragMenu = null)}>Cancelar</button>
          <button onclick={() => { const menu = dragMenu; dragMenu = null; fail(async () => { await call("switch_branch", { name: menu.source }); await call("rebase_branch", { upstream: menu.branch }); await loadRepo(); }); }}>Rebase</button>
          <button class="primary" onclick={() => { const menu = dragMenu; dragMenu = null; fail(async () => { await call("switch_branch", { name: menu.branch }); await call("merge", { branch: menu.source }); await loadRepo(); }); }}>Merge</button>
        </div>
      </div>
    </div>
  {/if}

  {#if modal}
    <div class="modal-back">
      <div class="modal">
        <p class="kicker">Confirmar</p>
        <p>{modal.text}</p>
        <div class="actions">
          <button onclick={() => (modal = null)}>Cancelar</button>
          <button class="primary" onclick={() => { const action = modal.action; modal = null; fail(action); }}>Confirmar</button>
        </div>
      </div>
    </div>
  {/if}
</div>
