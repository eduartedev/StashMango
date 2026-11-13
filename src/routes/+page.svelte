<script>
  import { open } from "@tauri-apps/plugin-dialog";
  import { call } from "$lib/api";
  import { branchMenuItems, menuPosition } from "$lib/branchMenu.js";
  import { settingsNav, settingsSectionOr } from "$lib/settingsNav.js";
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
  let branchMenu = $state(null);
  let steps = $state([]);
  let openSections = $state({ ramas: true, worktrees: true, stash: true, prs: true });
  let settingsSection = $state("perfiles");
  let closedFolders = $state({});
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
        branchMenu = null;
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

  function setPassphrase(value) {
    passphrase = value;
    if (!info?.path) return;
    phrases = { ...phrases, [info.path]: value };
  }

  function openBranchMenu(event, branch) {
    event.preventDefault();
    const current = branch === (info?.branch || "");
    const place = menuPosition(event.clientX, event.clientY, 248, current ? 188 : 188, {
      width: window.innerWidth,
      height: window.innerHeight,
    });
    branchMenu = { branch, current, ...place };
  }

  async function copyBranchName(name) {
    await navigator.clipboard.writeText(name);
    notice = "Nombre copiado.";
  }

  $effect(() => {
    if (!branchMenu) return;
    const close = (event) => {
      if (event.key === "Escape") branchMenu = null;
    };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  });

  function runBranchAction(id) {
    const menu = branchMenu;
    if (!menu) return;
    const branch = menu.branch;
    branchMenu = null;
    if (id === "copy") {
      fail(() => copyBranchName(branch));
    } else if (id === "switch") {
      fail(async () => {
        await call("switch_branch", { name: branch });
        await loadRepo();
      });
    } else if (id === "merge") {
      ask(`Merge de ${branch} en ${info.branch}.`, async () => {
        await call("merge", { branch });
        await loadRepo();
      });
    } else if (id === "rebase") {
      ask(`Rebase de ${info.branch} sobre ${branch}. Reescribe los commits de la rama actual.`, async () => {
        await call("rebase_branch", { upstream: branch });
        await loadRepo();
      });
    } else if (id === "fetch" || id === "pull" || id === "push") {
      fail(() => call("network", { action: id, passphrase: passphrase || null }).then(loadRepo));
    }
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

  function toggleSection(id) {
    openSections = { ...openSections, [id]: !openSections[id] };
  }

  function toggleFolder(path) {
    closedFolders = { ...closedFolders, [path]: !closedFolders[path] };
  }

  function branchTree(names) {
    const root = { children: [], leaves: [] };
    const index = new Map();
    for (const name of [...names].sort((a, b) => a.localeCompare(b))) {
      const parts = name.split("/").filter(Boolean);
      if (parts.length <= 1) {
        root.leaves.push(name);
        continue;
      }
      let parent = root;
      for (let i = 0; i < parts.length - 1; i++) {
        const path = parts.slice(0, i + 1).join("/");
        let child = index.get(path);
        if (!child) {
          child = { name: parts[i], path, children: [], leaves: [] };
          index.set(path, child);
          parent.children.push(child);
        }
        parent = child;
      }
      parent.leaves.push(name);
    }
    return root;
  }

  function leafCount(node) {
    return node.leaves.length + node.children.reduce((sum, child) => sum + leafCount(child), 0);
  }

  function branchRows(names, closed) {
    const rows = [];
    function walk(node, depth) {
      for (const child of node.children) {
        const shut = closed[child.path] === true;
        rows.push({
          kind: "folder",
          path: child.path,
          name: child.name,
          depth,
          shut,
          count: leafCount(child),
        });
        if (!shut) walk(child, depth + 1);
      }
      for (const branch of node.leaves) {
        rows.push({ kind: "leaf", branch, depth, label: branch.includes("/") ? branch.split("/").pop() : branch });
      }
    }
    walk(branchTree(names), 0);
    return rows;
  }

  let branchList = $derived(branchRows(branches, closedFolders));

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

{#snippet icon(name)}
  <svg class="ico" viewBox="0 0 16 16" aria-hidden="true">
    {#if name === "branch"}
      <circle cx="4" cy="3" r="1.3" /><circle cx="4" cy="13" r="1.3" /><circle cx="11" cy="13" r="1.3" />
      <path d="M4 4.3v7.4M4 5.2h4.2a2.4 2.4 0 0 1 2.4 2.4V11.7" />
    {:else if name === "fetch"}
      <path d="M13 8.2a5 5 0 1 1-1.4-3.4" /><path d="M13.2 2.6v3.2H10" />
    {:else if name === "pull"}
      <path d="M8 2.4v8.2M5.2 7.6 8 10.6l2.8-3M3.2 13.6h9.6" />
    {:else if name === "push"}
      <path d="M8 13.6V5.4M5.2 8.2 8 5.2l2.8 3M3.2 2.4h9.6" />
    {:else if name === "sliders"}
      <path d="M2.4 4h11.2M2.4 8h11.2M2.4 12h11.2" /><circle cx="6" cy="4" r="1.3" /><circle cx="10.5" cy="8" r="1.3" /><circle cx="7" cy="12" r="1.3" />
    {:else if name === "key"}
      <path d="M8.1 5.4a2.3 2.3 0 1 0 4.6 0 2.3 2.3 0 0 0-4.6 0M8.6 6.8 3.2 12.2V14h2.1v-1.5H7V11l1.6-1.6" />
    {:else if name === "worktree"}
      <rect x="2.2" y="2.6" width="7.2" height="7.2" rx="1.2" /><rect x="6.6" y="6.2" width="7.2" height="7.2" rx="1.2" />
    {:else if name === "stash"}
      <path d="M3 6.4h10v6.8H3zM3 6.4 8 3.2l5 3.2M8 3.2v10" />
    {:else if name === "pr"}
      <circle cx="4" cy="3.4" r="1.3" /><circle cx="4" cy="12.6" r="1.3" /><circle cx="12" cy="12.6" r="1.3" />
      <path d="M4 4.7v6.6M12 12.6V6.8A2.2 2.2 0 0 0 9.8 4.6H6.4" /><path d="M8.2 2.8 6.2 4.6l2 1.8" />
    {:else if name === "copy"}
      <rect x="5.2" y="5.2" width="7.4" height="7.4" rx="1.2" /><path d="M10.6 5.2V3.8A1.2 1.2 0 0 0 9.4 2.6H3.8A1.2 1.2 0 0 0 2.6 3.8v5.6A1.2 1.2 0 0 0 3.8 10.6H5.2" />
    {:else if name === "switch"}
      <path d="M4 3.2h6.2M8.2 1.4 10.2 3.2 8.2 5M12 12.8H5.8M7.8 11 5.8 12.8l2 1.8" />
    {:else if name === "merge"}
      <circle cx="4" cy="3.2" r="1.2" /><circle cx="12" cy="3.2" r="1.2" /><circle cx="8" cy="12.8" r="1.2" />
      <path d="M4 4.4v2.2a3 3 0 0 0 3 3h1M12 4.4v2.2a3 3 0 0 1-3 3H8v2" />
    {:else if name === "rebase"}
      <path d="M3.2 12.8h9.6M8 10.6V3.4M5.4 6 8 3.4 10.6 6" />
    {:else if name === "profile"}
      <circle cx="8" cy="5" r="2.1" /><path d="M3.4 13.4c.7-2.5 2.4-3.8 4.6-3.8s3.9 1.3 4.6 3.8" />
    {:else if name === "token"}
      <rect x="2.2" y="3.8" width="11.6" height="8.4" rx="1.4" /><path d="M2.2 7h11.6M5 10h2.6" />
    {/if}
  </svg>
{/snippet}

<div class="app" class:repo={screen === "repo"}>
  <header class="top">
    <button class="brand" onclick={() => (screen = "home")}>{@render mango(28)} StashMango</button>
    {#if screen === "repo" && info}
      <div class="dock">
        <span class="chip" title={info.branch || "HEAD"}>
          <span class="chip-kicker">{@render icon("branch")} Rama</span>
          <span class="chip-name">{info.branch || "HEAD"}</span>
        </span>
        <span class="dock-rule" aria-hidden="true"></span>
        <button onclick={() => fail(() => call("network", { action: "fetch", passphrase: passphrase || null }).then(loadRepo))}>{@render icon("fetch")} Fetch</button>
        <button onclick={() => fail(() => call("network", { action: "pull", passphrase: passphrase || null }).then(loadRepo))}>{@render icon("pull")} Pull</button>
        <button class="primary" onclick={() => fail(() => call("network", { action: "push", passphrase: passphrase || null }).then(loadRepo))}>{@render icon("push")} Push</button>
      </div>
    {/if}
    <span class="spacer"></span>
    <button class="ghost" onclick={openSettings}>{@render icon("sliders")} Configuración</button>
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
    <section class="settings-screen">
      <aside class="settings-nav">
        <p class="kicker">Configuración</p>
        {#each settingsNav as item (item.id)}
          <button type="button" class="nav-item" class:current={settingsSection === item.id} onclick={() => (settingsSection = settingsSectionOr(item.id))}>
            {@render icon(item.icon)}
            {item.label}
          </button>
        {/each}
      </aside>
      <div class="settings-pane">
        {#if settingsSection === "perfiles"}
          <article class="plate">
            <div class="plate-head">
              <h2>{@render icon("profile")} Perfiles</h2>
              <p>El perfil default ya existe. Aquí van el nombre y el correo. No se puede borrar.</p>
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
            <div class="plate-foot spread">
              <button onclick={startProfile}>Otro perfil</button>
              <button class="primary" onclick={() => fail(saveCurrent)}>Guardar</button>
            </div>
          </article>
        {:else if settingsSection === "servicios"}
          <article class="plate">
            <div class="plate-head">
              <h2>{@render icon("token")} Servicios</h2>
              <p>Tokens de {profileForm.label || "este perfil"}. El campo vacío deja el token que ya está en el llavero.</p>
            </div>
            <div class="services">
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
            <div class="plate-foot">
              <button class="primary" onclick={() => fail(saveCurrent)}>Guardar</button>
            </div>
          </article>
        {:else if settingsSection === "ssh"}
          <article class="plate">
            <div class="plate-head">
              <h2>{@render icon("key")} Frase SSH</h2>
              <p>Es de este repo y solo vive mientras la app está abierta. No se escribe en el disco ni en el perfil.</p>
            </div>
            {#if info}
              <div class="fields">
                <label>
                  {repoName(info.path)}
                  <input class="phrase" type="password" autocomplete="new-password" spellcheck="false" placeholder="Frase de la clave" value={passphrase} oninput={(event) => setPassphrase(event.currentTarget.value)} />
                </label>
              </div>
            {:else}
              <div class="fields">
                <p class="muted">Abre un repo. Cada uno guarda la suya.</p>
              </div>
            {/if}
          </article>
        {:else if settingsSection === "git"}
          <article class="plate">
            <div class="plate-head">
              <h2>{@render icon("branch")} Git</h2>
              <p>Apagado de inicio. Solo agrega la carpeta de StashMango al final del PATH de tu usuario.</p>
            </div>
            <div class="plate-foot split">
              <button onclick={() => fail(() => call("set_external_git", { enabled: true }))}>Usar este Git fuera de StashMango</button>
              <button onclick={() => fail(() => call("set_external_git", { enabled: false }))}>Quitarlo del PATH</button>
            </div>
          </article>
        {/if}
      </div>
    </section>
  {:else if info}
    <div class="work">
      <aside class="side">
        {#snippet fold(id, title, glyph)}
          <button class="fold-head" onclick={() => toggleSection(id)} aria-expanded={openSections[id]}>
            <span class="caret">{openSections[id] ? "▾" : "▸"}</span>
            {@render icon(glyph)}
            {title}
          </button>
        {/snippet}
        <div class="section">
          {@render fold("ramas", "Ramas", "branch")}
          {#if openSections.ramas}
            {#each branchList as row (row.kind + (row.path || row.branch))}
              {#if row.kind === "folder"}
                <button type="button" class="folder" class:shut={row.shut} style:padding-left={`${8 + row.depth * 12}px`} onclick={() => toggleFolder(row.path)}>
                  <span class="caret">{row.shut ? "▸" : "▾"}</span>
                  <span>{row.name}</span>
                  <span class="muted">{row.count}</span>
                </button>
              {:else}
                <button
                  type="button"
                  class="branch"
                  class:current={row.branch === info.branch}
                  style:padding-left={`${8 + row.depth * 12}px`}
                  draggable="true"
                  ondragstart={(event) => event.dataTransfer?.setData("text/plain", `branch:${row.branch}`)}
                  ondragover={(event) => event.preventDefault()}
                  ondrop={(event) => { event.preventDefault(); dropOn(row.branch, event.dataTransfer?.getData("text/plain") || ""); }}
                  onclick={() => fail(async () => { await call("switch_branch", { name: row.branch }); await loadRepo(); })}
                  oncontextmenu={(event) => openBranchMenu(event, row.branch)}
                ><span>{row.label}</span></button>
              {/if}
            {/each}
            <div class="rowish">
              <input placeholder="nueva rama" bind:value={newBranch} />
              <button onclick={() => fail(async () => { await call("create_branch", { name: newBranch }); newBranch = ""; await loadRepo(); })}>Crear</button>
            </div>
          {/if}
        </div>
        <div class="section">
          {@render fold("worktrees", "Worktrees", "worktree")}
          {#if openSections.worktrees}
            {#each worktrees as tree}
              <button class="branch" onclick={() => choose(tree.path)}><span>{tree.branch || tree.head.slice(0, 7)}</span><small class="muted">{tree.path}</small></button>
              {#if tree.path !== info.path}
                <button class="danger" onclick={() => ask(`Quitar el worktree ${tree.path}. Si hay cambios, git se niega.`, async () => { await call("remove_worktree", { path: tree.path, confirm: true }); await loadRepo(); })}>Quitar</button>
              {/if}
            {/each}
            <button onclick={async () => { const path = await open({ directory: true }); if (path && newBranch) await fail(async () => { await call("add_worktree", { path, branch: newBranch }); await loadRepo(); }); }}>Añadir en carpeta</button>
          {/if}
        </div>
        <div class="section">
          {@render fold("stash", "Stash", "stash")}
          {#if openSections.stash}
            {#each stashes as stash}<p class="mono">{stash}</p>{/each}
            <button onclick={() => fail(async () => { await call("stash_push", { message: message || "stash" }); await loadRepo(); })}>Guardar</button>
            <button onclick={() => fail(async () => { await call("stash_pop"); await loadRepo(); })}>Pop</button>
            <button class="danger" onclick={() => ask("Soltar el stash más reciente.", async () => { await call("stash_drop", { confirm: true }); await loadRepo(); })}>Drop</button>
          {/if}
        </div>
        <div class="section">
          {@render fold("prs", "Pull requests", "pr")}
          {#if openSections.prs}
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
          {/if}
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

  {#if branchMenu}
    <div
      class="context-back"
      role="presentation"
      onclick={() => (branchMenu = null)}
      oncontextmenu={(event) => { event.preventDefault(); branchMenu = null; }}
      onkeydown={(event) => { if (event.key === "Escape") branchMenu = null; }}
    >
      <div
        class="context"
        role="menu"
        tabindex="-1"
        style:left={`${branchMenu.x}px`}
        style:top={`${branchMenu.y}px`}
        onclick={(event) => event.stopPropagation()}
        onkeydown={(event) => { if (event.key === "Escape") branchMenu = null; }}
      >
        <p class="kicker">{branchMenu.current ? "Rama actual" : "Otra rama"}</p>
        <p class="context-name">{branchMenu.branch}</p>
        {#each branchMenuItems(branchMenu.current) as item (item.id)}
          <button type="button" role="menuitem" onclick={() => runBranchAction(item.id)}>{@render icon(item.icon)} {item.label}</button>
        {/each}
      </div>
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
