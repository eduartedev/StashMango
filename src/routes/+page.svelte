<script>
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { LogicalSize } from "@tauri-apps/api/window";
  import { call } from "$lib/api";
  import { captionToggleTarget } from "$lib/caption.js";
  import { nextSide, sideNav } from "$lib/sideNav.js";
  import { branchMenuItems, menuPosition } from "$lib/branchMenu.js";
  import { refChips } from "$lib/refChips.js";
  import { branchRows, localBranchNames } from "$lib/branchList.js";
  import { assertGitflow, defaultGitflow, gitflowCreateKinds, gitflowKinds, normalizeGitflow } from "$lib/gitflow.js";
  import { clampColumn, graphColumnChoices, layoutPayload, normalizeColumns, toggleGraphColumn } from "$lib/columns.js";
  import { defaultTemplates, fillTemplate, insertToken, normalizeTemplates, sampleVars, templateKinds, templateTokens, templateVars, wrapToken } from "$lib/commitTemplates.js";
  import { settingsNav, settingsSectionOr } from "$lib/settingsNav.js";
  import { defaultPreferences, normalizePreferences, branchVisibilityOptions, descriptionVisibilityOptions, groupColorOptions } from "$lib/preferences.js";
  import { emptyServiceSaved, emptyServiceTokens, gitHadTokenNotice, services, tokenPage } from "$lib/services.js";
  import { defaultRepoLive, gitHooksDoc } from "$lib/gitHooks.js";
  import { BUSY_DELAY_MS, ERROR_MS, NOTICE_MS } from "$lib/flash.js";
  import { branchStamp, keepLog, restoreLog, shouldReloadLog } from "$lib/commitCache.js";
  import { assertClone, cloneNameFromUrl, joinDest } from "$lib/startRepo.js";
  import { stashRefOk, stashRows } from "$lib/stashList.js";
  import { diffHunks, lineKind, stageableIndex } from "$lib/diffHunks.js";
  import { canStagePreview, previewImageSrc, previewKicker } from "$lib/previewFile.js";
  import { fileFolder, fileName, groupWorkFiles } from "$lib/workFiles.js";
  import { placeRepoTab } from "$lib/repoTabs.js";
  import { sameWorktreePath, worktreeFolder, worktreeLabel } from "$lib/worktreeCard.js";
  import Graph from "$lib/Graph.svelte";
  import Merge from "$lib/Merge.svelte";

  let screen = $state("home");
  let error = $state("");
  let notice = $state("");
  let busyCount = $state(0);
  let showBusy = $state(false);
  let recents = $state([]);
  let tabs = $state([]);
  let activePath = $state("");
  let drafts = $state({});
  let phrases = $state({});
  let info = $state(null);
  let commits = $state([]);
  let openLanes = $state([]);
  let done = $state(false);
  const logCache = new Map();
  let loadBusy = false;
  let loadAgain = false;
  let branchRefs = $state([]);
  let worktrees = $state([]);
  let stashes = $state([]);
  let stashRefs = $state([]);
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
  let branchInput;
  let gitflowAction = $state(null);
  let prTitle = $state("");
  let prHead = $state("");
  let prBase = $state("main");
  let modal = $state(null);
  let dragMenu = $state(null);
  let branchMenu = $state(null);
  let commitMenu = $state(null);
  let branchFrom = $state("");
  let logText = $state(null);
  let logLoading = $state(false);
  let steps = $state([]);
  let sideView = $state("ramas");
  let columns = $state(normalizeColumns());
  let settingsSection = $state("perfiles");
  let templates = $state(normalizeTemplates());
  let templateKind = $state("merge");
  let templateBox;
  let closedFolders = $state({});
  let gitflow = $state(defaultGitflow());
  let repoLive = $state(defaultRepoLive());
  let branchQuery = $state("");
  let homeForm = $state("");
  let cloneUrl = $state("");
  let cloneDest = $state("");
  let profileForm = $state({ id: "default", label: "default", name: "", email: "" });
  let preferences = $state(normalizePreferences());
  let serviceTokens = $state(emptyServiceTokens());
  let serviceSaved = $state(emptyServiceSaved());

  async function boot() {
    try {
      recents = await call("recents");
      columns = normalizeColumns(await call("layout"));
      templates = normalizeTemplates(await call("templates"));
      preferences = normalizePreferences(await call("preferences"));
    } catch (err) {
      const text = String(err);
      if (!text.includes("invoke")) error = text;
    }
  }
  boot();

  async function fail(run, opts = {}) {
    error = "";
    const quiet = Boolean(opts.quiet);
    const first = busyCount === 0;
    let delay;
    if (!quiet) {
      busyCount += 1;
      if (first) delay = setTimeout(() => { showBusy = true; }, BUSY_DELAY_MS);
    }
    try {
      return await run();
    } catch (err) {
      const text = String(err);
      if (!text.includes("invoke")) error = text;
    } finally {
      if (!quiet) {
        busyCount = Math.max(0, busyCount - 1);
        if (busyCount === 0) {
          clearTimeout(delay);
          showBusy = false;
        }
      }
    }
  }

  $effect(() => {
    const text = notice;
    if (!text) return;
    const timer = setTimeout(() => { if (notice === text) notice = ""; }, NOTICE_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    const text = error;
    if (!text) return;
    const timer = setTimeout(() => { if (error === text) error = ""; }, ERROR_MS);
    return () => clearTimeout(timer);
  });

  function rememberLog(path, stamp) {
    if (!path) return;
    logCache.set(path, {
      commits,
      openLanes,
      done,
      stamp: stamp ?? logCache.get(path)?.stamp ?? "",
    });
  }

  function showCachedLog(path) {
    const hit = restoreLog(logCache, path);
    commits = hit.commits;
    openLanes = hit.openLanes;
    done = hit.done;
  }

  async function loadRepo() {
    if (loadBusy) {
      loadAgain = true;
      return;
    }
    loadBusy = true;
    try {
      do {
        loadAgain = false;
        await fillRepo();
      } while (loadAgain);
    } finally {
      loadBusy = false;
    }
  }

  async function fillRepo() {
    const path = info?.path;
    info = await call("refresh");
    branchRefs = await call("branches");
    gitflow = normalizeGitflow(await call("repo_gitflow"));
    repoLive = await call("repo_live");
    worktrees = await call("worktrees");
    stashes = await call("stash_list");
    stashRefs = await call("stash_refs");
    profile = await call("active_profile");
    remote = await call("remote");
    profiles = await call("profiles");
    const shas = await call("branch_shas");
    if (shas && Array.isArray(shas)) {
      branchToSha = new Map(shas);
    }
    const stamp = branchStamp(branchRefs);
    const cached = path ? logCache.get(path) : null;
    if (shouldReloadLog(cached, stamp)) {
      const page = await call("log_page", { skip: 0, openLanes: [] });
      const next = keepLog(cached || { commits, openLanes, done }, page);
      if (!next.kept) {
        commits = next.commits;
        openLanes = next.openLanes;
        done = next.done;
      }
    } else if (cached && commits !== cached.commits) {
      commits = cached.commits;
      openLanes = cached.openLanes;
      done = cached.done;
    }
    rememberLog(info?.path || path, stamp);
    touchTab();
  }

  function repoName(path) {
    const parts = path.split(/[\\/]/).filter(Boolean);
    return parts[parts.length - 1] || path;
  }

  function touchTab() {
    if (!info) return;
    tabs = placeRepoTab(tabs, info, worktrees, preferences.autoGroupWorktrees);
    activePath = info.path;
  }

  async function enterRepo(opened) {
    if (activePath) {
      drafts = { ...drafts, [activePath]: message };
      phrases = { ...phrases, [activePath]: passphrase };
    }
    const same = opened.path === activePath;
    info = opened;
    screen = "repo";
    homeForm = "";
    touchTab();
    if (!same) {
      closedFolders = {};
      branchQuery = "";
      selected = null;
      files = [];
      diff = null;
      conflict = null;
      steps = [];
      prs = [];
      showCachedLog(opened.path);
      dragMenu = null;
      branchMenu = null;
      message = drafts[info.path] || fillTemplate(templates.commit, workVars()) || "";
      passphrase = phrases[info.path] || "";
    }
    await loadRepo();
  }

  async function choose(path) {
    await fail(async () => {
      const opened = await call("open_repo", { path });
      await enterRepo(opened);
    });
  }

  function goHome() {
    screen = "home";
    homeForm = "";
    fail(async () => {
      recents = await call("recents");
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
      goHome();
      return;
    }
    await choose(neighbor.path);
  }

  async function browse() {
    const path = await open({ directory: true });
    if (path) await choose(path);
  }

  async function createRepo() {
    homeForm = "";
    const path = await open({ directory: true });
    if (!path) return;
    await fail(async () => {
      const opened = await call("init_repo", { path });
      await enterRepo(opened);
    });
  }

  async function pickCloneDest() {
    const path = await open({ directory: true });
    if (!path) return;
    cloneDest = joinDest(path, cloneNameFromUrl(cloneUrl));
  }

  async function cloneNow() {
    const payload = assertClone(cloneUrl, cloneDest);
    const opened = await call("clone_repo", payload);
    await enterRepo(opened);
  }

  async function more() {
    if (done) return;
    const page = await call("log_page", { skip: commits.length, openLanes });
    commits = [...commits, ...page.commits];
    openLanes = page.open_lanes;
    done = page.done;
    rememberLog(info?.path);
  }

  function onScroll(event) {
    const el = event.currentTarget;
    if (el.scrollTop + el.clientHeight > el.scrollHeight - 40) more();
  }

  async function pickCommit(commit) {
    selected = commit;
    conflict = null;
    diff = null;
    files = commit.pending
      ? commit.pendingCurrent === false
        ? []
        : (info?.files || []).map((file) => file.path)
      : await call("commit_files", { sha: commit.id });
  }

  function previewWorkFile(path) {
    const meta = (info?.files || []).find((file) => file.path === path);
    return showPreview(path, { cached: Boolean(meta?.staged) && !meta?.unstaged });
  }

  async function showPreview(path, options = {}) {
    const cached = Boolean(options.cached);
    const sha = options.sha || "";
    conflict = null;
    if (!sha) selected = null;
    diff = await call("diff_file", { path, cached, sha: sha || null });
  }

  async function stageHunk(index) {
    await fail(async () => {
      await call("apply_selection", { path: diff.path, cached: diff.cached, hunks: [index], lines: [] });
      await loadRepo();
      await showPreview(diff.path, { cached: diff.cached, sha: diff.sha });
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
      await showPreview(diff.path, { cached: diff.cached, sha: diff.sha });
    });
  }

  async function stageDiffLine(hunk, line) {
    const pick = stageableIndex(diffHunks(diff.text)[hunk]?.lines || [], line);
    if (pick < 0) return;
    await stageLine(hunk, pick);
  }

  function ask(text, action) {
    modal = { text, action };
  }

  function workVars(extra = {}) {
    return templateVars({
      current: extra.current || info?.branch || "HEAD",
      branch: extra.branch || extra.current || info?.branch || "",
      files: extra.files || info?.files || [],
      sha: extra.sha || selected?.id || "",
      summary: extra.summary || selected?.summary || "",
    });
  }

  function mergeMessage(incoming) {
    return fillTemplate(templates.merge, workVars({ branch: incoming || "esta rama" }));
  }

  async function runMerge(incoming) {
    const filled = mergeMessage(incoming);
    await call("merge", { branch: incoming, message: filled });
    await loadRepo();
    const hasConflict = (info?.files || []).some((file) => file.conflict);
    message = hasConflict ? filled : fillTemplate(templates.commit, workVars()) || "";
  }

  function putToken(id) {
    const key = templateKind;
    const current = templates[key] || "";
    const start = templateBox?.selectionStart ?? current.length;
    const end = templateBox?.selectionEnd ?? start;
    const next = insertToken(current, id, start, end);
    templates = { ...templates, [key]: next.text };
    queueMicrotask(() => {
      templateBox?.focus();
      templateBox?.setSelectionRange(next.cursor, next.cursor);
    });
  }

  function setPassphrase(value) {
    passphrase = value;
    if (!info?.path) return;
    phrases = { ...phrases, [info.path]: value };
  }

  function openBranchMenu(event, branch, remote = false) {
    event.preventDefault();
    const current = branch === (info?.branch || "");
    const items = branchMenuItems(current, branch, info?.branch || "").filter((item) => !(remote && item.id === "switch"));
    const height = Math.min(600, 80 + items.length * 44);
    const place = menuPosition(event.clientX, event.clientY, 280, height, {
      width: document.documentElement.clientWidth,
      height: document.documentElement.clientHeight,
    });
    branchMenu = { branch, current, remote, ...place };
  }

  function openCommitMenu(event, commit) {
    event.preventDefault();
    const items = commitMenuItems(commit);
    const height = Math.min(600, 80 + items.length * 44);
    const place = menuPosition(event.clientX, event.clientY, 300, height, {
      width: document.documentElement.clientWidth,
      height: document.documentElement.clientHeight,
    });
    commitMenu = { commit, ...place };
  }

  function commitMenuItems(commit) {
    const items = [
      { id: "copy-hash", label: "Copiar hash", icon: "copy" },
      { id: "copy-msg", label: "Copiar mensaje", icon: "copy" },
      { id: "branch", label: "Nueva rama aquí", icon: "plus" },
      { id: "tag", label: "Crear etiqueta aquí", icon: "folder" },
      { id: "cherry", label: "Cherry-pick", icon: "play" },
      { id: "reset", label: "Reset hard aquí", icon: "undo", danger: true },
    ];
    const current = info?.branch || "";
    const chips = refChips(commit?.refs);
    const namesFromRefs = chips.filter((chip) => chip.kind === "local" && chip.label !== "HEAD").map((chip) => chip.label);
    const namesFromMap = [...branchToSha.entries()].filter(([_, sha]) => sha === commit.id).map(([name]) => name);
    const names = [...new Set([...namesFromRefs, ...namesFromMap])];
    for (const name of names.slice(0, 3)) {
      items.push({ id: `name:${name}`, label: `Copiar nombre · ${name}`, icon: "copy" });
      if (name === current) continue;
      items.push({ id: `switch:${name}`, label: `Cambiar a ${name}`, icon: "switch" });
      items.push({ id: `merge:${name}`, label: `Merge ${name} en ${current || "actual"}`, icon: "merge" });
      items.push({ id: `rebase:${name}`, label: `Rebase ${current || "actual"} sobre ${name}`, icon: "rebase" });
      items.push({ id: `delete:${name}`, label: `Eliminar ${name}`, icon: "close", danger: true });
    }
    return items;
  }

  async function runCommitAction(id) {
    const commit = commitMenu?.commit;
    commitMenu = null;
    if (!commit) return;
    const sep = id.indexOf(":");
    const action = sep < 0 ? id : id.slice(0, sep);
    const arg = sep < 0 ? "" : id.slice(sep + 1);
    if (action === "name") {
      await navigator.clipboard.writeText(arg);
      notice = `Copiado ${arg}`;
      return;
    }
    if (action === "switch") {
      fail(async () => {
        await call("switch_branch", { name: arg });
        await loadRepo();
      });
      return;
    }
    if (action === "merge") {
      ask(`Merge de ${arg} en ${info?.branch}.`, () => runMerge(arg));
      return;
    }
    if (action === "rebase") {
      const text = fillTemplate(templates.rebase, workVars({ branch: arg })) || `Rebase de ${info?.branch} sobre ${arg}. Reescribe los commits de la rama actual.`;
      ask(text, async () => {
        await call("rebase_branch", { upstream: arg });
        await loadRepo();
      });
      return;
    }
    if (action === "delete") {
      ask(`Eliminar la rama ${arg}. Se perderá si no está en ningún remoto.`, async () => {
        await call("delete_branch", { name: arg });
        await loadRepo();
      });
      return;
    }
    if (action === "tag") {
      await ask("Nombre de la etiqueta (ej: v1.0.0)", async (name) => {
        if (!name.trim()) return;
        await call("create_tag", { name: name.trim(), message: null });
        await loadRepo();
      });
      return;
    }
    if (id === "copy-hash") {
      await navigator.clipboard.writeText(commit.id);
      notice = `Copiado ${commit.id.slice(0, 7)}`;
      return;
    }
    if (id === "copy-msg") {
      await navigator.clipboard.writeText(commit.summary || "");
      notice = "Mensaje copiado";
      return;
    }
    if (id === "branch") {
      branchFrom = commit.id;
      newBranch = "";
      notice = `Nueva rama desde ${commit.id.slice(0, 7)}. Escribe el nombre y pulsa Crear.`;
      requestAnimationFrame(() => branchInput?.focus());
      return;
    }
    if (id === "cherry") {
      await ask(`Cherry-pick de ${commit.id.slice(0, 7)} (${commit.summary}) en ${info?.branch || "la rama actual"}.`, async () => {
        await call("cherry_pick", { sha: commit.id, message: "" });
        await loadRepo();
      });
      return;
    }
    if (id === "reset") {
      await ask(`Reset hard a ${commit.id.slice(0, 7)}. El trabajo sin commit se pierde. Puedes volver con «Volver».`, async () => {
        await call("reset_hard", { target: commit.id, confirm: true });
        await loadRepo();
      });
    }
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
      ask(`Merge de ${branch} en ${info.branch}.`, () => runMerge(branch));
    } else if (id === "rebase") {
      const text = fillTemplate(templates.rebase, workVars({ branch })) || `Rebase de ${info.branch} sobre ${branch}. Reescribe los commits de la rama actual.`;
      ask(text, async () => {
        await call("rebase_branch", { upstream: branch });
        await loadRepo();
      });
    } else if (id === "delete") {
      ask(`Eliminar la rama ${branch}. Se perderá si no está en ningún remoto.`, async () => {
        await call("delete_branch", { name: branch });
        await loadRepo();
      });
    } else if (id === "tag") {
      ask("Nombre de la etiqueta (ej: v1.0.0)", async (name) => {
        if (!name.trim()) return;
        await call("create_tag", { name: name.trim(), message: null });
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
      const picked = commits.find((commit) => commit.id === sha);
      const filled = fillTemplate(templates.cherry, workVars({ current: branch, sha, summary: picked?.summary || "" }));
      ask(filled || `Cherry-pick de ${sha.slice(0, 7)} en ${branch}.`, async () => {
        await call("switch_branch", { name: branch });
        await call("cherry_pick", { sha, message: filled });
        await loadRepo();
      });
    }
  }

  function blankTokens() {
    serviceTokens = emptyServiceTokens();
  }

  async function openTokenPage(host) {
    const url = tokenPage(host);
    if (!url) return;
    await openUrl(url);
  }

  async function fillTokenFromGit(service) {
    const token = await call("fill_git_token", { host: service.host });
    serviceTokens = { ...serviceTokens, [service.host]: token };
    notice = gitHadTokenNotice(service.label);
  }

  async function editProfile(item) {
    profileForm = { id: item.id, label: item.label, name: item.name, email: item.email };
    blankTokens();
    const saved = emptyServiceSaved();
    for (const service of services) {
      saved[service.host] = await call("has_token", { profileId: item.id, host: service.host });
    }
    serviceSaved = saved;
  }

  async function loadSettings() {
    profiles = await call("profiles");
    const current = profiles.profiles.find((item) => item.id === profileForm.id) || profiles.profiles[0];
    if (current) await editProfile(current);
    if (info?.path) {
      gitflow = normalizeGitflow(await call("repo_gitflow"));
      repoLive = await call("repo_live");
    }
  }

  function openSettings() {
    screen = screen === "settings" ? (info ? "repo" : "home") : "settings";
    if (screen === "settings") fail(loadSettings);
  }

  function startProfile() {
    profileForm = { id: crypto.randomUUID(), label: "", name: "", email: "" };
    blankTokens();
    serviceSaved = emptyServiceSaved();
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

  function saveColumns() {
    fail(() => call("save_layout", { layout: layoutPayload(columns) }));
  }

  function setGraphColumn(id) {
    columns = { ...columns, graphColumns: toggleGraphColumn(columns.graphColumns, id) };
    saveColumns();
  }

  function setColumn(name, open) {
    columns = { ...columns, [name]: open };
    saveColumns();
  }

  function dragColumn(event, key) {
    const startX = event.clientX;
    const start = columns[key];
    const sign = key === "detailWidth" ? -1 : 1;
    const handle = event.currentTarget;
    handle.setPointerCapture?.(event.pointerId);
    const move = (ev) => {
      columns = { ...columns, [key]: clampColumn(key, start + sign * (ev.clientX - startX)) };
    };
    const up = () => {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", up);
      saveColumns();
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", up);
  }

  function pickSide(id) {
    const next = nextSide(columns.side, sideView, id);
    sideView = next.view;
    if (next.open !== columns.side) setColumn("side", next.open);
  }

  function toggleFolder(path) {
    const shut = closedFolders[path] !== false;
    closedFolders = { ...closedFolders, [path]: !shut };
  }

  function pickKind(prefix) {
    newBranch = prefix;
    branchInput?.focus();
  }

  async function createBranch() {
    const name = newBranch.trim();
    if (!name) return;
    if (branchFrom) {
      await call("create_branch_at", { name, sha: branchFrom });
      branchFrom = "";
    } else {
      await call("create_branch", { name });
    }
    newBranch = "";
    await loadRepo();
  }

  async function openLog() {
    logLoading = true;
    try {
      logText = await call("repo_log", { limit: 300 });
    } catch (err) {
      notice = String(err);
    } finally {
      logLoading = false;
    }
  }

  function openGitflowPanel() {
    gitflowAction = { kind: "panel" };
  }

  function closeGitflowPanel() {
    gitflowAction = null;
  }

  async function gitflowStart(kind) {
    const prefix = kind === "feature" ? "feature/" : kind === "release" ? "release/" : "hotfix/";
    newBranch = prefix;
    branchInput?.focus();
    gitflowAction = null;
  }

  async function gitflowFinish(kind) {
    const current = info?.branch || "";
    if (!current) return;
    const text = kind === "feature" ? `Terminar feature ${current}` : kind === "release" ? `Terminar release ${current}` : `Terminar hotfix ${current}`;
    ask(text, async () => {
      await call("switch_branch", { name: "develop" });
      await call("merge", { branch: current, message: text });
      await loadRepo();
    });
    gitflowAction = null;
  }

  async function saveGitflow(patch = {}) {
    const flow = assertGitflow({ ...gitflow, ...patch });
    gitflow = normalizeGitflow(await call("set_repo_gitflow", { flow }));
    closedFolders = {};
  }

  let branches = $derived(localBranchNames(branchRefs));
  let branchToSha = $state(new Map());
  let branchSections = $derived(branchRows(branchRefs, closedFolders, { gitflow, query: branchQuery }));
  let createKinds = $derived(gitflow.enabled ? gitflowCreateKinds(gitflow) : []);
  let stashList = $derived(stashRows(stashes));

  async function saveStash() {
    const filled = (fillTemplate(templates.stash, workVars({ summary: message })) || message || "stash").replace(/\s+/g, " ").trim();
    await call("stash_push", { message: filled });
    await loadRepo();
  }

  async function runStash(kind, id) {
    if (!stashRefOk(id)) throw new Error("Ese stash no es válido.");
    if (kind === "drop") {
      stashes = await call("stash_drop", { id, confirm: true });
      await loadRepo();
      return;
    }
    await call(kind === "apply" ? "stash_apply" : "stash_pop", { id });
    await loadRepo();
  }

  let maximized = $state(false);

  function frameWindow() {
    try {
      return getCurrentWindow();
    } catch {
      return null;
    }
  }

  function syncFrame() {
    const win = frameWindow();
    if (!win) return;
    win.isMaximized().then((value) => {
      maximized = value;
    }).catch(() => {});
  }

  $effect(() => {
    let alive = true;
    let unlisten = () => {};
    let watchTimer;
    let pendingKind = "";
    listen("repo-changed", (event) => {
      if (!alive || screen !== "repo" || !info || conflict || modal) return;
      pendingKind = event.payload === "git" || pendingKind === "git" ? "git" : "worktree";
      clearTimeout(watchTimer);
      watchTimer = setTimeout(() => {
        const kind = pendingKind;
        pendingKind = "";
        fail(async () => {
          if (kind === "git") await loadRepo();
          else info = await call("refresh");
        }, { quiet: true });
      }, 160);
    }).then((stop) => {
      if (alive) unlisten = stop;
      else stop();
    }).catch(() => {});
    return () => {
      alive = false;
      clearTimeout(watchTimer);
      unlisten();
    };
  });

  $effect(() => {
    const win = frameWindow();
    if (!win) return;
    let alive = true;
    let unlisten = () => {};
    syncFrame();
    win.onResized(() => {
      if (alive) syncFrame();
    }).then((stop) => {
      if (alive) unlisten = stop;
      else stop();
    }).catch(() => {});
    return () => {
      alive = false;
      unlisten();
    };
  });

  function minimizeWindow() {
    frameWindow()?.minimize().catch(() => {});
  }

  function toggleWindow() {
    const win = frameWindow();
    if (!win) return;
    win.toggleMaximize().then(syncFrame).catch(() => {});
  }

  function closeWindow() {
    frameWindow()?.close().catch(() => {});
  }

  function toggleFromCaption(event) {
    if (!captionToggleTarget(event.target)) return;
    toggleWindow();
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

{#snippet mangoWalk()}
  <svg class="mango-walk" viewBox="0 0 80 100" aria-hidden="true">
    <g class="mango-arm left">
      <path d="M24 40c-8 6-12 12-11 18" stroke="#c47a12" stroke-width="6" stroke-linecap="round" fill="none"/>
      <g class="mango-hand">
        <circle cx="12" cy="58" r="5.2" fill="#e8941a"/>
        <ellipse cx="8.2" cy="55.5" rx="2.2" ry="2.8" fill="#e8941a"/>
      </g>
    </g>
    <g class="mango-arm right">
      <path d="M56 40c8 6 12 12 11 18" stroke="#c47a12" stroke-width="6" stroke-linecap="round" fill="none"/>
      <g class="mango-hand">
        <circle cx="68" cy="58" r="5.2" fill="#e8941a"/>
        <ellipse cx="71.8" cy="55.5" rx="2.2" ry="2.8" fill="#e8941a"/>
      </g>
    </g>
    <g class="mango-body" transform="translate(8 4)">
      <path d="M32 58c-14 0-22-12-20-26 2-16 14-24 20-26 6 2 18 10 20 26 2 14-6 26-20 26z" fill="#F5B942"/>
      <path d="M32 54c-8 0-14-8-12-18 1-8 7-14 12-16 5 2 11 8 12 16 2 10-4 18-12 18z" fill="#E8941A"/>
      <path d="M32 8c1 7 6 11 12 12-7 1-11 5-12 11-1-6-5-10-12-11 6-1 11-5 12-12z" fill="#8EA34A"/>
      <path d="M14 29h-5" stroke="#14110e" stroke-width="2.2" stroke-linecap="round"/>
      <path d="M50 29h5" stroke="#14110e" stroke-width="2.2" stroke-linecap="round"/>
      <rect x="15" y="25" width="14" height="9.5" rx="3" fill="#14110e"/>
      <rect x="35" y="25" width="14" height="9.5" rx="3" fill="#14110e"/>
      <path d="M29 29.2h6" stroke="#14110e" stroke-width="2.4" stroke-linecap="round"/>
      <path d="M17.5 27.2h5" stroke="#fff" stroke-opacity="0.28" stroke-width="1.6" stroke-linecap="round"/>
      <path d="M37.5 27.2h5" stroke="#fff" stroke-opacity="0.28" stroke-width="1.6" stroke-linecap="round"/>
    </g>
    <g class="mango-leg left">
      <path d="M35 58v20" stroke="#c47a12" stroke-width="7" stroke-linecap="round"/>
      <ellipse cx="32" cy="80" rx="7.5" ry="3.4" fill="#a86810"/>
    </g>
    <g class="mango-leg right">
      <path d="M45 58v20" stroke="#c47a12" stroke-width="7" stroke-linecap="round"/>
      <ellipse cx="48" cy="80" rx="7.5" ry="3.4" fill="#a86810"/>
    </g>
  </svg>
{/snippet}

{#snippet brand(name)}
  <svg class="service-logo" viewBox="0 0 24 24" aria-hidden="true">
    {#if name === "github"}
      <path fill="#f0f6fc" d="M12 .3C5.37.3 0 5.67 0 12.3c0 5.3 3.44 9.8 8.2 11.39.6.11.82-.26.82-.58 0-.28-.01-1.04-.02-2.04-3.34.73-4.04-1.61-4.04-1.61-.55-1.39-1.34-1.76-1.34-1.76-1.09-.74.08-.73.08-.73 1.2.09 1.84 1.24 1.84 1.24 1.07 1.83 2.81 1.3 3.5 1 .11-.78.42-1.31.76-1.61-2.67-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.13-.3-.54-1.52.12-3.18 0 0 1.01-.32 3.3 1.23a11.5 11.5 0 0 1 6 0c2.3-1.55 3.3-1.23 3.3-1.23.66 1.66.25 2.88.12 3.18.77.84 1.23 1.91 1.23 3.22 0 4.61-2.81 5.62-5.48 5.92.43.37.81 1.1.81 2.22 0 1.61-.01 2.9-.01 3.29 0 .32.22.69.83.57C20.56 22.1 24 17.6 24 12.3 24 5.67 18.63.3 12 .3z"/>
    {:else if name === "gitlab"}
      <path fill="#E24329" d="M12 21.2 3.7 15.2 2.2 10.5h5.3z"/>
      <path fill="#FC6D26" d="M12 21.2 7.5 8.7H2.2L3.7 15.2z"/>
      <path fill="#FCA326" d="M7.5 8.7 5.7 3.2a.4.4 0 0 0-.76 0L2.2 8.7h5.3z"/>
      <path fill="#E24329" d="m12 21.2 8.3-6 1.5-4.7h-5.3z"/>
      <path fill="#FC6D26" d="m12 21.2 4.5-12.5h5.3l-1.5 6.5z"/>
      <path fill="#FCA326" d="m16.5 8.7 1.8-5.5a.4.4 0 0 1 .76 0l2.74 5.5H16.5z"/>
      <path fill="#FC6D26" d="M7.5 8.7h9L12 21.2z"/>
    {:else if name === "bitbucket"}
      <path fill="#2684FF" d="M.78 1.21a.77.77 0 0 0-.77.89l3.26 19.81c.09.5.52.87 1.03.87H19.95a.77.77 0 0 0 .77-.64l3.27-20.04a.77.77 0 0 0-.77-.89zm13.74 14.32H9.52L8.17 8.47h7.56z"/>
    {/if}
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
    {:else if name === "link"}
      <path d="M9 3.2h3.8V7M12.8 3.4 7.4 8.8M6.2 4.4H3.6A1.2 1.2 0 0 0 2.4 5.6v6.8A1.2 1.2 0 0 0 3.6 13.6h6.8a1.2 1.2 0 0 0 1.2-1.2V9.6" />
    {:else if name === "template"}
      <rect x="3.4" y="2.4" width="9.2" height="11.2" rx="1.2" />
      <path d="M5.4 5.6h5.2M5.4 8h5.2M5.4 10.4h3.2" />
    {:else if name === "maximize"}
      <rect x="3.2" y="3.2" width="9.6" height="9.6" rx="1.2" />
    {:else if name === "restore"}
      <rect x="5.2" y="2.8" width="8" height="8" rx="1" />
      <path d="M2.8 5.2V13h7.8" />
    {:else if name === "close"}
      <path d="M4 4l8 8M12 4l-8 8" />
    {:else if name === "folder"}
      <path d="M2.4 4.2h4l1.4 1.6h6.2v7.6H2.4z" />
    {:else if name === "plus"}
      <path d="M8 3.2v9.6M3.2 8h9.6" />
    {:else if name === "minus"}
      <path d="M3.2 8h9.6" />
    {:else if name === "check"}
      <path d="M3.2 8.4 6.4 11.6 12.8 4.4" />
    {:else if name === "remote"}
      <path d="M8 2.6a5.4 5.4 0 1 0 0 10.8 5.4 5.4 0 0 0 0-10.8z" />
      <path d="M2.6 8h10.8M8 2.8c1.6 1.6 2.4 3.3 2.4 5.2s-.8 3.6-2.4 5.2c-1.6-1.6-2.4-3.3-2.4-5.2s.8-3.6 2.4-5.2" />
    {:else if name === "play"}
      <path d="M5 3.4v9.2L12.6 8z" />
    {:else if name === "undo"}
      <path d="M3 6.4h7.4a3.4 3.4 0 0 1 0 6.8H7.4M5.4 3.8 3 6.4l2.4 2.6" />
    {/if}
  </svg>
{/snippet}

<div class="app" class:repo={screen === "repo"} class:side-shut={screen === "repo" && !columns.side} style:--side="{columns.sideWidth}px">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header class="top" data-tauri-drag-region ondblclick={toggleFromCaption}>
    <button class="brand" onclick={goHome}>{@render mango(28)} StashMango</button>
    <span class="spacer" data-tauri-drag-region></span>
    {#if screen === "repo"}
      <button class="ghost" onclick={openLog} disabled={logLoading}>{logLoading ? "Cargando…" : "Historial"}</button>
    {/if}
    <button class="ghost" onclick={openSettings}>{@render icon("sliders")} Configuración</button>
    <div class="win-controls">
      <button type="button" aria-label="Minimizar" onclick={minimizeWindow}>{@render icon("minus")}</button>
      <button type="button" aria-label={maximized ? "Restaurar" : "Maximizar"} onclick={toggleWindow}>{@render icon(maximized ? "restore" : "maximize")}</button>
      <button type="button" class="close" aria-label="Cerrar" onclick={closeWindow}>{@render icon("close")}</button>
    </div>
  </header>
  {#if screen === "repo" && info}
  <header class="top-secondary">
    {#if tabs.length}
      <div class="tabs">
        {#each tabs as tab (tab.id)}
          <div class="tab" class:active={tab.path === activePath && screen === "repo"}>
            <button class="tab-main" onclick={() => focusTab(tab.path)}>
              <span>{tab.name}</span>
              <small>{tab.branch || "HEAD"}</small>
            </button>
            <button class="x" aria-label="Cerrar {tab.name}" onclick={() => closeTab(tab.path)}>×</button>
          </div>
        {/each}
        <button class="tab-add" type="button" title="Más" aria-label="Más" onclick={goHome}>+</button>
      </div>
    {/if}
    <div class="profile-selector">
      <select aria-label="Perfil de git" onchange={(event) => {
        const value = event.target.value;
        if (value === "new") {
          settingsSection = "perfiles";
          openSettings();
          event.target.value = profile?.id || "";
          return;
        }
        fail(() => call("set_repo_profile", { profileId: value || null }).then(() => call("active_profile")).then((val) => profile = val));
      }}>
        <option value="new">Agregar perfil</option>
        {#each profiles.profiles as item}
          <option value={item.id} selected={profile?.id === item.id}>{item.label}</option>
        {/each}
      </select>
    </div>
  </header>
  <header class="top-tertiary">
    <div class="dock">
      <span class="chip" title={info.branch || "HEAD"}>
        {#if preferences.showToolbarLabels}
          <span class="chip-kicker">{@render icon("branch")} Rama</span>
        {/if}
        <span class="chip-name">{info.branch || "HEAD"}</span>
      </span>
      <span class="dock-rule" aria-hidden="true"></span>
      <button onclick={() => fail(() => call("network", { action: "fetch", passphrase: passphrase || null }).then(loadRepo))}>{@render icon("fetch")}{#if preferences.showToolbarLabels} Fetch{/if}</button>
      <button onclick={() => fail(() => call("network", { action: "pull", passphrase: passphrase || null }).then(loadRepo))}>{@render icon("pull")}{#if preferences.showToolbarLabels} Pull{/if}</button>
      <button class="primary" onclick={() => fail(() => call("network", { action: "push", passphrase: passphrase || null }).then(loadRepo))}>{@render icon("push")}{#if preferences.showToolbarLabels} Push{/if}</button>
    </div>
  </header>
  {/if}
  <div class="shell">
    {#if screen === "repo" && info}
    <aside class="side" class:shut={!columns.side}>
      <div class="side-body">
      <nav class="side-nav" aria-label="Paneles del repo">
        {#each sideNav as item (item.id)}
          <button
            type="button"
            class:on={columns.side && sideView === item.id}
            aria-pressed={columns.side && sideView === item.id}
            aria-label={item.label}
            title={item.label}
            onclick={() => pickSide(item.id)}
          >{@render icon(item.icon)}</button>
        {/each}
      </nav>
      {#if columns.side}
      <div class="side-pane">
      <button type="button" class="splitter" aria-label="Ancho del panel" onpointerdown={(event) => dragColumn(event, "sideWidth")}></button>
      {#if sideView === "ramas"}
      <div class="section branches">
        <div class="branches-head">
          <p class="kicker">Ramas</p>
          <span class="branches-total muted">{branchRefs.length}</span>
        </div>
        <input class="branch-search" type="search" placeholder="Buscar rama" bind:value={branchQuery} autocomplete="off" spellcheck="false" aria-label="Buscar rama" />
          {#if branchQuery.trim() && !branchSections.some((s) => s.rows.length)}
            <p class="muted">Ninguna rama con ese nombre.</p>
          {/if}
          {#if !branchSections.some((s) => s.rows.length) && !branchQuery.trim()}
            <p class="muted">No hay ramas.</p>
          {/if}
          {#each branchSections as section (section.id)}
            {#if section.rows.length}
              <div class="branch-section">
                <button type="button" class="section-header" onclick={() => toggleFolder(section.id)}>
                  <span class="caret">{closedFolders[section.id] !== false ? "▸" : "▾"}</span>
                  <span class="section-label">{section.label}</span>
                  <span class="muted">{section.rows.reduce((sum, row) => sum + (row.count || 1), 0)}</span>
                </button>
                {#if closedFolders[section.id] !== false}
                {:else}
                  {#each section.rows as row (row.kind + (row.path || row.branch))}
                    {#if row.kind === "folder"}
                      <button type="button" class="folder {row.kindId ? `gf-${row.kindId}` : ''}" class:shut={row.shut} style:padding-left={`${16 + row.depth * 12}px`} onclick={() => toggleFolder(row.path)}>
                        <span class="caret">{row.shut ? "▸" : "▾"}</span>
                        {#if row.kindId}<span class="kind-dot"></span>{/if}
                        <span class="fname">{row.name}</span>
                        {#if row.remote}<span class="remote-tag">{@render icon("remote")}</span>{/if}
                        <span class="muted">{row.count}</span>
                      </button>
                    {:else}
                      <button
                        type="button"
                        class="branch"
                        class:current={row.branch === info.branch}
                        class:remote={row.remote}
                        style:padding-left={`${16 + row.depth * 12}px`}
                        draggable="true"
                        ondragstart={(event) => event.dataTransfer?.setData("text/plain", `branch:${row.branch}`)}
                        ondragover={(event) => event.preventDefault()}
                        ondrop={(event) => { event.preventDefault(); dropOn(row.branch, event.dataTransfer?.getData("text/plain") || ""); }}
                        onclick={() => { if (row.remote) return; fail(async () => { await call("switch_branch", { name: row.branch }); await loadRepo(); }); }}
                        oncontextmenu={(event) => openBranchMenu(event, row.branch, row.remote)}
                        title={row.synced ? `Igual que origin/${row.branch}` : row.remote ? row.branch : row.branch === info.branch ? "Rama actual" : row.branch}
                      >
                        <span class="name">{row.label}</span>
                        {#if row.remote || row.synced}
                          <span class="sync">{@render icon("remote")}</span>
                        {/if}
                      </button>
                    {/if}
                  {/each}
                {/if}
              </div>
            {/if}
          {/each}
          <div class="branch-new">
            {#if gitflow.enabled}
              <button type="button" class="gitflow-toggle" onclick={openGitflowPanel}>
                {@render icon("merge")} Gitflow
              </button>
            {/if}
            {#if createKinds.length}
              <div class="branch-kinds">
                {#each createKinds as kind (kind.id)}
                  <button type="button" class="kind" title={`Nueva rama ${kind.prefix}…`} onclick={() => pickKind(kind.prefix)}>{@render icon("plus")}{kind.label}</button>
                {/each}
              </div>
            {/if}
            <div class="rowish">
              <input bind:this={branchInput} placeholder="nueva rama" bind:value={newBranch} onkeydown={(event) => { if (event.key === "Enter") fail(createBranch); }} aria-label="Nueva rama" />
              <button disabled={!newBranch.trim()} onclick={() => fail(createBranch)}>Crear</button>
            </div>
          </div>
      </div>
      {:else if sideView === "worktrees"}
      <div class="section">
        <p class="kicker">Worktrees</p>
          <div class="trees">
            {#each worktrees as tree (tree.path)}
              {@const here = sameWorktreePath(tree.path, info.path)}
              <div class="tree" class:here>
                <button
                  type="button"
                  class="tree-open"
                  title={tree.path}
                  aria-current={here ? "page" : undefined}
                  onclick={() => { if (!here) choose(tree.path); }}
                >
                  <span class="tree-branch">{worktreeLabel(tree) || "HEAD"}</span>
                  <span class="tree-folder">{worktreeFolder(tree.path)}</span>
                </button>
                {#if here}
                  <span class="here-mark" title="Estás aquí">{@render icon("check")}</span>
                {:else}
                  <button
                    type="button"
                    class="tree-drop"
                    title="Quitar worktree"
                    aria-label="Quitar worktree {worktreeFolder(tree.path)}"
                    onclick={() => ask(`Quitar el worktree ${tree.path}. Si hay cambios, git se niega.`, async () => { await call("remove_worktree", { path: tree.path, confirm: true }); await loadRepo(); })}
                  >{@render icon("close")}</button>
                {/if}
              </div>
            {/each}
            <div class="tree-foot">
              <input placeholder="rama" bind:value={newBranch} aria-label="Rama del worktree" />
              <button
                type="button"
                class="tree-add"
                disabled={!newBranch.trim()}
                title={newBranch.trim() ? "Elegir carpeta" : "Escribe la rama primero"}
                onclick={async () => {
                  const path = await open({ directory: true });
                  if (path && newBranch.trim()) await fail(async () => { await call("add_worktree", { path, branch: newBranch.trim() }); await loadRepo(); });
                }}
              >{@render icon("plus")} Carpeta</button>
            </div>
          </div>
      </div>
      {:else if sideView === "stash"}
      <div class="section">
        <p class="kicker">Stash</p>
        <div class="stashes">
          {#if !stashList.length}
            <p class="muted">No hay nada guardado.</p>
          {/if}
          {#each stashList as item (item.id)}
            <article class="stash">
              <div class="stash-head">
                <span class="stash-id">{item.id}</span>
                {#if item.branch}<span class="stash-branch">{item.branch}</span>{/if}
              </div>
              <p class="stash-title">{item.title}</p>
              <div class="stash-actions">
                <button type="button" onclick={() => fail(() => runStash("apply", item.id))}>Apply</button>
                <button type="button" class="primary" onclick={() => fail(() => runStash("pop", item.id))}>Pop</button>
                <button type="button" class="danger" onclick={() => ask(`Quitar ${item.id}.`, () => runStash("drop", item.id))}>Quitar</button>
              </div>
            </article>
          {/each}
          <div class="stash-foot">
            <button type="button" class="primary" onclick={() => fail(saveStash)}>Guardar</button>
          </div>
        </div>
      </div>
      {:else if sideView === "prs"}
      <div class="section">
        <p class="kicker">Pull requests</p>
          <button onclick={() => fail(async () => { prs = await call("list_prs"); })}>Actualizar</button>
          {#each prs as pr}
            <p>{pr.title}<span class="muted"> {pr.head} → {pr.base} · {pr.author}</span></p>
            <button onclick={() => fail(async () => { diff = { path: pr.title, text: await call("pr_diff", { number: pr.number }), too_big: false, binary: false, cached: false, hunks: 0 }; })}>Diff</button>
            <button onclick={() => fail(() => call("merge_pr", { number: pr.number }))}>Merge</button>
          {/each}
          <input placeholder="título" bind:value={prTitle} />
          <input placeholder="head" bind:value={prHead} />
          <input placeholder="base" bind:value={prBase} />
          <button onclick={() => fail(() => { const title = prTitle.trim() || fillTemplate(templates.pr, workVars({ branch: prHead, summary: message || selected?.summary || "" })); return call("create_pr", { title, head: prHead, base: prBase }); })}>Crear PR</button>
      </div>
      {/if}
      </div>
      {/if}
      {#if gitflowAction && gitflowAction.kind === "panel"}
        <div class="gitflow-panel-back" onclick={closeGitflowPanel}>
          <div class="gitflow-panel" onclick={(event) => event.stopPropagation()}>
            <div class="gitflow-panel-head">
              <p class="kicker">Gitflow</p>
              <button type="button" class="ghost" onclick={closeGitflowPanel}>{@render icon("close")}</button>
            </div>
            <div class="gitflow-panel-body">
              <div class="gitflow-column">
                <p class="gf-col-head">Start</p>
                <button type="button" class="gf-btn" onclick={() => gitflowStart("feature")}>Feature</button>
                <button type="button" class="gf-btn" onclick={() => gitflowStart("release")}>Release</button>
                <button type="button" class="gf-btn" onclick={() => gitflowStart("hotfix")}>Hotfix</button>
              </div>
              <div class="gitflow-panel-vline"></div>
              <div class="gitflow-column">
                <p class="gf-col-head">Finish</p>
                <button type="button" class="gf-btn" onclick={() => gitflowFinish("feature")}>Feature</button>
                <button type="button" class="gf-btn" onclick={() => gitflowFinish("release")}>Release</button>
                <button type="button" class="gf-btn" onclick={() => gitflowFinish("hotfix")}>Hotfix</button>
              </div>
            </div>
          </div>
        </div>
      {/if}
    </aside>
    {/if}
    <div class="stage">
  {#if error}<button type="button" class="banner error" title="Clic para cerrar" onclick={() => (error = "")}>{error}</button>{/if}
  {#if notice}<button type="button" class="banner ok" title="Clic para cerrar" onclick={() => (notice = "")}>{notice}</button>{/if}

  {#if screen === "home"}
    <section class="home">
      <div class="hero">
        {@render mango(92)}
        <div>
          <p class="kicker">Puesto de noche</p>
          <h1>Repos recientes</h1>
          <p class="lede">Abre, clona o crea.</p>
        </div>
      </div>
      <div class="start-actions">
        <button type="button" class="start-act" onclick={browse}>
          {@render icon("folder")}
          <strong>Abrir</strong>
          <small>Una carpeta que ya es git</small>
        </button>
        <button type="button" class="start-act" class:on={homeForm === "clone"} onclick={() => (homeForm = homeForm === "clone" ? "" : "clone")}>
          {@render icon("pull")}
          <strong>Clonar</strong>
          <small>Desde una URL o un repo local</small>
        </button>
        <button type="button" class="start-act" onclick={createRepo}>
          {@render icon("plus")}
          <strong>Crear</strong>
          <small>Init en una carpeta vacía</small>
        </button>
      </div>
      {#if homeForm === "clone"}
        <div class="start-form">
          <label>
            URL
            <input bind:value={cloneUrl} placeholder="https://github.com/org/repo.git" spellcheck="false" />
          </label>
          <label>
            Destino
            <span class="start-dest">
              <input bind:value={cloneDest} placeholder="Carpeta nueva" spellcheck="false" />
              <button type="button" onclick={pickCloneDest}>Elegir</button>
            </span>
          </label>
          <div class="start-form-foot">
            <button type="button" onclick={() => (homeForm = "")}>Cancelar</button>
            <button class="primary" onclick={() => fail(cloneNow)}>Clonar</button>
          </div>
        </div>
      {/if}
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
              <div class="profile-list">
                <h3>Perfiles existentes</h3>
                <div class="profile-items">
                  {#each profiles.profiles as item}
                    <div class="profile-item" class:active={item.id === profileForm.id}>
                      <button class="profile-select" onclick={() => fail(() => editProfile(item))}>
                        <strong>{item.label}</strong>
                        <span class="muted">{item.name ? `${item.name} <${item.email}>` : "Sin firma todavía"}</span>
                      </button>
                      {#if item.id !== "default"}
                        <button class="danger" onclick={() => fail(async () => { profiles = await call("delete_profile", { id: item.id }); if (profileForm.id === item.id) await editProfile(profiles.profiles.find((profile) => profile.id === "default")); })}>Borrar</button>
                      {/if}
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
            <div class="profile-form">
              <h3>{profileForm.id === "default" ? "Editar perfil" : "Nuevo perfil"}</h3>
              <div class="fields">
                <label>Etiqueta <input bind:value={profileForm.label} /></label>
                <label>Nombre <input bind:value={profileForm.name} /></label>
                <label>Correo <input bind:value={profileForm.email} /></label>
              </div>
              <div class="plate-foot spread">
                <button onclick={startProfile}>Nuevo perfil</button>
                <button class="primary" onclick={() => fail(saveCurrent)}>Guardar</button>
              </div>
            </div>
          </article>
        {:else if settingsSection === "servicios"}
          <article class="plate services-plate">
            <div class="plate-head">
              <h2>{@render icon("token")} Servicios</h2>
              <p>Tokens de {profileForm.label || "este perfil"}. El campo vacío deja el del llavero. Crear token abre la página; Desde Git usa el que git ya tenga.</p>
            </div>
            <div class="services">
              {#each services as service (service.host)}
                <div class="service">
                  <span class="service-mark" data-brand={service.id} aria-hidden="true">{@render brand(service.id)}</span>
                  <div class="service-head">
                    <div>
                      <strong>{service.label}</strong>
                      <span class="host">{service.host}</span>
                      <span class="hint">{service.hint}</span>
                    </div>
                    <span class="pill" class:on={serviceSaved[service.host]}>{serviceSaved[service.host] ? "Guardado" : "Sin token"}</span>
                  </div>
                  <label class="service-field">
                    <span>Token</span>
                    <input type="password" placeholder="Pegar token" bind:value={serviceTokens[service.host]} autocomplete="off" spellcheck="false" />
                  </label>
                  <div class="service-actions">
                    <button type="button" onclick={() => fail(() => openTokenPage(service.host))}>{@render icon("link")} Crear token</button>
                    <button type="button" onclick={() => fail(() => fillTokenFromGit(service))}>{@render icon("branch")} Desde Git</button>
                    {#if serviceSaved[service.host]}
                      <button type="button" class="danger" onclick={() => fail(async () => { await call("clear_token", { profileId: profileForm.id, host: service.host }); serviceSaved = { ...serviceSaved, [service.host]: false }; })}>Quitar</button>
                    {/if}
                  </div>
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
        {:else if settingsSection === "plantillas"}
          {@const kind = templateKinds.find((item) => item.id === templateKind) || templateKinds[0]}
          {@const preview = fillTemplate(templates[kind.id], sampleVars(info, info?.files)) || "Git escribe el mensaje solo."}
          <article class="plate templates-plate">
            <div class="plate-head">
              <h2>{@render icon("template")} Plantillas</h2>
              <p>Un tipo a la vez. Pulsa un hueco para insertarlo. El ejemplo usa tu rama actual si hay un repo abierto.</p>
            </div>
            <div class="tpl-shell">
              <div class="tpl-rail" role="tablist" aria-label="Tipos de plantilla">
                {#each templateKinds as item (item.id)}
                  <button
                    type="button"
                    role="tab"
                    aria-selected={templateKind === item.id}
                    class="tpl-item"
                    class:current={templateKind === item.id}
                    onclick={() => (templateKind = item.id)}
                  >
                    {@render icon(item.icon)}
                    <span>
                      <strong>{item.label}</strong>
                      <small>{item.hint}</small>
                    </span>
                  </button>
                {/each}
              </div>
              <div class="tpl-editor">
                <div class="tpl-head">
                  <p class="kicker">{kind.label}</p>
                  <p class="muted">{kind.hint}</p>
                </div>
                <textarea
                  rows="5"
                  bind:this={templateBox}
                  bind:value={templates[kind.id]}
                  placeholder={defaultTemplates[kind.id] || "Vacío: no se usa plantilla"}
                  spellcheck="false"
                ></textarea>
                <div class="tpl-chips">
                  {#each templateTokens as token (token.id)}
                    <button
                      type="button"
                      class="tpl-chip"
                      class:on={(templates[kind.id] || "").includes(wrapToken(token.id))}
                      title={token.label}
                      onclick={() => putToken(token.id)}
                    >{wrapToken(token.id)}</button>
                  {/each}
                </div>
                <div class="tpl-preview">
                  <p class="kicker">Así queda</p>
                  <p>{preview}</p>
                </div>
              </div>
            </div>
            <div class="plate-foot spread">
              <button type="button" onclick={() => (templates = { ...templates, [kind.id]: defaultTemplates[kind.id] })}>Restablecer esta</button>
              <button class="primary" onclick={() => fail(async () => { templates = normalizeTemplates(await call("save_templates", { templates })); notice = "Plantillas guardadas."; })}>Guardar</button>
            </div>
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
          <article class="plate">
            <div class="plate-head">
              <h2>{@render icon("merge")} Git Flow</h2>
              <p>Por defecto: main, master, develop, feature/, release/, hotfix/, support/. Nombres sueltos o prefijos con / al final, separados por coma.</p>
            </div>
            {#if info}
              <div class="column-picks">
                <label class="column-pick">
                  <input type="checkbox" checked={gitflow.enabled} onchange={(event) => fail(() => saveGitflow({ enabled: event.currentTarget.checked }))} />
                  <strong>Usar Git Flow</strong>
                  <small>{repoName(info.path)}</small>
                </label>
              </div>
              <div class="fields gitflow">
                {#each gitflowKinds as kind (kind.id)}
                  <label>
                    {kind.label}
                    <input
                      value={gitflow[kind.id]}
                      placeholder={kind.def}
                      spellcheck="false"
                      oninput={(event) => (gitflow = { ...gitflow, [kind.id]: event.currentTarget.value })}
                    />
                    <span class="hint">{kind.hint}. Vacío usa {kind.def}.</span>
                  </label>
                {/each}
              </div>
              <div class="plate-foot spread">
                <button type="button" onclick={() => (gitflow = { ...defaultGitflow(), enabled: gitflow.enabled })}>Restablecer ramas</button>
                <button class="primary" onclick={() => fail(async () => { await saveGitflow(); notice = "Git Flow guardado."; })}>Guardar</button>
              </div>
            {:else}
              <div class="fields">
                <p class="muted">Abre un repo. Cada uno guarda la suya.</p>
              </div>
            {/if}
          </article>
          <article class="plate">
            <div class="plate-head">
              <h2>{@render icon("template")} Git Hooks</h2>
              <p>Scripts que Git corre al commit, merge o push. Esta carpeta vale para este repo y sus worktrees.</p>
            </div>
            {#if info}
              <div class="fields">
                <label>
                  Carpeta de hooks
                  <span class="start-dest">
                    <input value={repoLive.hooksDir} spellcheck="false" oninput={(event) => (repoLive = { ...repoLive, hooksDir: event.currentTarget.value })} />
                    <button type="button" onclick={() => fail(async () => { const path = await open({ directory: true }); if (path) repoLive = await call("set_repo_hooks_dir", { path }); })}>Elegir</button>
                  </span>
                </label>
                {#if repoLive.hooks.length}
                  <p class="hook-names">{repoLive.hooks.join(" · ")}</p>
                {:else}
                  <p class="muted">No hay scripts en esa carpeta (los .sample no cuentan).</p>
                {/if}
              </div>
              <div class="column-picks">
                <label class="column-pick">
                  <input type="checkbox" checked={repoLive.skipHooks} onchange={(event) => fail(async () => { repoLive = await call("set_repo_skip_hooks", { skip: event.currentTarget.checked }); })} />
                  <strong>Saltar hooks al hacer commit</strong>
                  <small>Equivale a --no-verify solo desde StashMango.</small>
                </label>
              </div>
              <div class="plate-foot spread">
                <button type="button" onclick={() => fail(() => openUrl(gitHooksDoc))}>Documentación de Git</button>
                <button class="primary" onclick={() => fail(async () => { repoLive = await call("set_repo_hooks_dir", { path: repoLive.hooksDir }); notice = "Hooks guardados."; })}>Guardar carpeta</button>
              </div>
            {:else}
              <div class="fields">
                <p class="muted">Abre un repo. Cada uno apunta a la suya.</p>
              </div>
            {/if}
          </article>
        {:else if settingsSection === "apariencia"}
          <article class="plate">
            <div class="plate-head">
              <h2>{@render icon("palette")} Apariencia</h2>
              <p>Personaliza cómo se muestra el grafo y la interfaz.</p>
            </div>
            <div class="prefs-section">
              <p class="section-label">Columnas del grafo</p>
              <div class="column-picks">
                {#each graphColumnChoices as item (item.id)}
                  <label class="column-pick">
                    <input
                      type="checkbox"
                      checked={columns.graphColumns.includes(item.id)}
                      onchange={() => setGraphColumn(item.id)}
                    />
                    <strong>{item.label}</strong>
                    <small>{item.hint}</small>
                  </label>
                {/each}
              </div>
            </div>
            <div class="prefs-section">
              <p class="section-label">Interfaz general</p>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.showToolbarLabels} />
                <span>Mostrar etiquetas en la barra de herramientas</span>
              </label>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.showAuthorInitials} />
                <span>Mostrar iniciales del autor en lugar de avatares</span>
              </label>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.showGhostBranch} />
                <span>Mostrar rama/etiqueta fantasma al hover en commits</span>
              </label>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.highlightBranchRows} />
                <span>Resaltar filas asociadas al hover en ramas</span>
              </label>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.autoGroupWorktrees} />
                <span>Agrupar automáticamente pestañas de worktrees</span>
              </label>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.showSpellCheck} />
                <span>Habilitar corrección ortográfica</span>
              </label>
            </div>
            <div class="prefs-section">
              <p class="section-label">Grafo de commits</p>
              <div class="pref-row">
                <label>Visibilidad de ramas:</label>
                <select bind:value={preferences.graphBranchVisibility}>
                  {#each branchVisibilityOptions as opt}
                    <option value={opt.id}>{opt.label}</option>
                  {/each}
                </select>
              </div>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.showBranchesInGraph} />
                <span>Mostrar ramas en el grafo</span>
              </label>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.showTagsInGraph} />
                <span>Mostrar etiquetas en el grafo</span>
              </label>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.showAuthorInGraph} />
                <span>Mostrar autor en el grafo</span>
              </label>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.showDateInGraph} />
                <span>Mostrar fecha en el grafo</span>
              </label>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.showMessageInGraph} />
                <span>Mostrar mensaje en el grafo</span>
              </label>
              <div class="pref-row">
                <label>Descripción en el grafo:</label>
                <select bind:value={preferences.showDescriptionInGraph}>
                  {#each descriptionVisibilityOptions as opt}
                    <option value={opt.id}>{opt.label}</option>
                  {/each}
                </select>
              </div>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.showShaInGraph} />
                <span>Mostrar SHA en el grafo</span>
              </label>
              <label class="pref-row">
                <input type="checkbox" bind:checked={preferences.showTreeInGraph} />
                <span>Mostrar árbol en el grafo</span>
              </label>
            </div>
            <div class="prefs-section">
              <p class="section-label">Colores</p>
              <div class="pref-row">
                <label>Índice de color del grafo:</label>
                <input type="number" min="0" max="15" bind:value={preferences.graphColorIndex} />
              </div>
              <div class="pref-row">
                <label>Color de grupos por defecto:</label>
                <select bind:value={preferences.defaultGroupColor}>
                  {#each groupColorOptions as opt}
                    <option value={opt.id}>{opt.label}</option>
                  {/each}
                </select>
              </div>
            </div>
            <div class="plate-actions">
              <button class="primary" onclick={() => fail(async () => { await call("save_preferences", { preferences }); notice = "Preferencias guardadas."; })}>Guardar</button>
            </div>
          </article>
        {/if}
      </div>
    </section>
  {:else if info}
    <div class="work" class:detail-shut={!columns.detail} style:--detail="{columns.detailWidth}px">
      <main class="graph-wrap" class:stage-open={Boolean((conflict && !conflict.too_big) || diff)} onscroll={onScroll}>
        {#if conflict && !conflict.too_big}
          <div class="merge-stage">
            <div class="merge-head">
              <p class="kicker">Conflicto</p>
              <h2 title={conflict.path}>{fileName(conflict.path) || conflict.path}</h2>
              <span class="muted">{fileFolder(conflict.path)}</span>
              <button type="button" class="ghost" onclick={() => (conflict = null)}>Volver al grafo</button>
            </div>
            <Merge
              path={conflict.path}
              ours={conflict.ours}
              theirs={conflict.theirs}
              bind:result={conflict.result}
              onsave={saveResult}
              onours={() => ask("Quedarme con el nuestro pisa el archivo.", async () => { await call("resolve_side", { path: conflict.path, ours: true, confirm: true }); conflict = null; await loadRepo(); })}
              ontheirs={() => ask("Quedarme con el suyo pisa el archivo.", async () => { await call("resolve_side", { path: conflict.path, ours: false, confirm: true }); conflict = null; await loadRepo(); })}
            />
          </div>
        {:else if diff}
          {@const hunks = diffHunks(diff.text)}
          {@const imageSrc = previewImageSrc(diff)}
          {@const staging = canStagePreview(diff)}
          <div class="merge-stage">
            <div class="merge-head">
              <p class="kicker">{previewKicker(diff)}</p>
              <h2 title={diff.path}>{fileName(diff.path) || diff.path}</h2>
              <span class="muted">{fileFolder(diff.path)}</span>
              <button type="button" class="ghost" onclick={() => (diff = null)}>Volver al grafo</button>
            </div>
            {#if imageSrc}
              <div class="image-preview">
                <img alt={diff.path} src={imageSrc} />
              </div>
            {:else if diff.kind === "text"}
              <pre class="file-preview">{diff.text}</pre>
            {:else if diff.too_big || diff.binary || diff.kind === "binary"}
              <div class="diff-board">
                <p>No se carga el archivo{diff.too_big ? " porque pasa el tope" : " porque es binario"}.</p>
                {#if !diff.sha}
                  <button onclick={() => fail(() => call("open_external", { path: diff.path }))}>Abrir afuera</button>
                {/if}
              </div>
            {:else if !hunks.length}
              <p class="muted">Sin líneas que mostrar.</p>
            {:else}
              <div class="diff-board">
                {#each hunks as hunk, hunkIndex}
                  <article class="hunk">
                    <header>
                      <code>{hunk.header}</code>
                      {#if staging}
                        <button type="button" onclick={() => stageHunk(hunkIndex)}>{diff.cached ? "Sacar hunk" : "Añadir hunk"}</button>
                      {/if}
                    </header>
                    {#each hunk.lines as line, lineIndex}
                      <div class="diff-line" class:add={lineKind(line) === "add"} class:del={lineKind(line) === "del"} class:ctx={lineKind(line) === "ctx"}>
                        {#if staging && stageableIndex(hunk.lines, lineIndex) >= 0}
                          <button
                            type="button"
                            class="work-toggle"
                            class:minus={diff.cached}
                            title={diff.cached ? "Sacar línea" : "Añadir línea"}
                            aria-label="{diff.cached ? "Sacar línea" : "Añadir línea"} {lineIndex + 1}"
                            onclick={() => stageDiffLine(hunkIndex, lineIndex)}
                          >{@render icon(diff.cached ? "minus" : "plus")}</button>
                        {:else}
                          <span class="line-gap"></span>
                        {/if}
                        <span class="mono">{line}</span>
                      </div>
                    {/each}
                  </article>
                {/each}
              </div>
            {/if}
          </div>
        {:else}
          <Graph
            {commits}
            selected={selected?.id || ""}
            onselect={pickCommit}
            refsWidth={columns.refsWidth}
            graphWidth={columns.graphWidth}
            sideOpen={columns.side}
            detailOpen={columns.detail}
            onToggle={(name) => setColumn(name, !columns[name])}
            onRefs={(width, save) => { columns = { ...columns, refsWidth: width }; if (save) saveColumns(); }}
            onGraph={(width, save) => { columns = { ...columns, graphWidth: width }; if (save) saveColumns(); }}
            fields={columns.graphColumns}
            {branches}
            current={info.branch || ""}
            onpick={(name) => fail(async () => { await call("switch_branch", { name }); await loadRepo(); })}
            onmenu={openCommitMenu}
            pending={info.files.length}
            {worktrees}
            repoPath={info.path}
            {stashRefs}
            pendingConflicts={info.files.filter((file) => file.conflict).length}
            {preferences}
          />
          {#if !done}<button onclick={more}>Más historial</button>{/if}
        {/if}
      </main>

      <aside class="detail" class:shut={!columns.detail}>
        {#if columns.detail}
        <button type="button" class="splitter detail-splitter" aria-label="Ancho del detalle" onpointerdown={(event) => dragColumn(event, "detailWidth")}></button>
        {:else}
        <button
          type="button"
          class="rail"
          aria-expanded="false"
          aria-label="Expandir detalle"
          onclick={() => setColumn("detail", true)}
        >
          {@render icon("sliders")}
          <span>Detalle</span>
        </button>
        {/if}
        {#if columns.detail}
        <div class="section desk">
          <h2>Trabajo</h2>
          {#if profile?.name || profile?.email}
          <div class="who-box">
            <p class="work-kicker">Identidad</p>
            <p class="who-mail">{profile.name}{profile.email ? ` · ${profile.email}` : ""}</p>
          </div>
          {/if}
          {#if !info.files.length}
            <p class="work-empty">Sin cambios en el directorio de trabajo.</p>
          {/if}
          {#each groupWorkFiles(info.files) as group (group.id)}
            <p class="work-kicker" class:conflict={group.id === "conflict"}>{group.title} <span>{group.files.length}</span></p>
            <div class="work-list">
              {#each group.files as file (group.id + file.path)}
                <div class="work-row" class:on={(!diff?.sha && diff?.path === file.path) || conflict?.path === file.path} class:conflict={file.conflict}>
                  <button
                    type="button"
                    class="work-file"
                    title={file.path}
                    onclick={() => file.conflict ? fail(async () => { conflict = await call("conflict_text", { path: file.path }); diff = null; selected = null; }) : fail(() => showPreview(file.path, { cached: group.id === "staged" }))}
                  >
                    <span class="work-name">{fileName(file.path) || file.path}</span>
                    {#if fileFolder(file.path)}<span class="work-dir">{fileFolder(file.path)}</span>{/if}
                  </button>
                  {#if group.id !== "conflict"}
                    <button
                      type="button"
                      class="work-toggle"
                      class:minus={group.id === "staged"}
                      title={group.id === "staged" ? "Sacar del índice" : "Añadir al índice"}
                      aria-label="{group.id === "staged" ? "Sacar del índice" : "Añadir al índice"} {file.path}"
                      onclick={() => fail(async () => { await call("stage_file", { path: file.path, stage: group.id !== "staged" }); await loadRepo(); })}
                    >{@render icon(group.id === "staged" ? "minus" : "plus")}</button>
                  {/if}
                </div>
              {/each}
            </div>
          {/each}
          <div class="commit-box">
            <textarea rows="3" placeholder="Mensaje del commit" bind:value={message}></textarea>
            <button class="primary" disabled={!message.trim()} onclick={() => fail(async () => { const filled = fillTemplate(message, workVars()) || message.trim(); await call("commit", { message: filled }); message = fillTemplate(templates.commit, workVars()) || ""; await loadRepo(); })}>Commit</button>
          </div>
        </div>
        {#if conflict?.too_big}
          <div class="section">
            <h2>Conflicto</h2>
            <p>El conflicto pasa el tope.</p>
            <button onclick={() => fail(() => call("open_external", { path: conflict.path }))}>Abrir afuera</button>
            <button onclick={() => fail(() => call("mergetool", { path: conflict.path }))}>Herramienta de merge</button>
          </div>
        {/if}
        <div class="section desk">
          <h2>Rebase visual</h2>
          <div class="rebase-load">
            <input placeholder="base, por ejemplo main" aria-label="Base del rebase" bind:value={prBase} />
            <button type="button" onclick={() => (steps = commits.slice(0, 12).map((commit) => ({ sha: commit.id, action: "pick", message: fillTemplate(templates.reword, workVars({ sha: commit.id, summary: commit.summary })) || commit.summary })))}>Cargar</button>
          </div>
          {#if steps.length}
            <div class="rebase-steps">
              {#each steps as step, index (step.sha + index)}
                <article class="rebase-step">
                  <div class="rebase-step-head">
                    <span class="stash-id">{step.sha.slice(0, 7)}</span>
                    <select bind:value={step.action} aria-label="Acción del rebase">
                      <option value="pick">pick</option>
                      <option value="drop">drop</option>
                      <option value="reword">reword</option>
                    </select>
                    <button type="button" disabled={index === 0} onclick={() => { const next = [...steps]; const swap = next[index - 1]; if (!swap) return; next[index - 1] = next[index]; next[index] = swap; steps = next; }}>Subir</button>
                  </div>
                  <input bind:value={step.message} aria-label="Mensaje del commit" />
                </article>
              {/each}
            </div>
          {/if}
          <button type="button" class="rebase-apply" disabled={!steps.length} onclick={() => ask("Esto reescribe la rama con la lista de arriba.", async () => { await call("replay", { base: prBase, steps: [...steps].reverse() }); steps = []; await loadRepo(); })}>Aplicar</button>
        </div>
        {#if selected}
          <div class="section desk">
            <h2>Commit</h2>
            <article class="commit-card">
              <h3 class="commit-title">{selected.summary}</h3>
              <p class="muted commit-meta">{selected.pending ? (selected.pendingCurrent === false ? selected.pendingLabel : "directorio de trabajo") : `${selected.author}${selected.refs ? ` · ${selected.refs}` : ""}`}</p>
              {#if files.length}
                <div class="work-list">
                  {#each files as file (file)}
                    <div class="work-row solo" class:on={diff?.path === file && (selected.pending ? !diff.sha : diff.sha === selected.id)}>
                      <button type="button" class="work-file" title={file} onclick={() => fail(() => selected.pending ? previewWorkFile(file) : showPreview(file, { sha: selected.id }))}>
                        <span class="work-name">{fileName(file) || file}</span>
                        {#if fileFolder(file)}<span class="work-dir">{fileFolder(file)}</span>{/if}
                      </button>
                    </div>
                  {/each}
                </div>
              {/if}
              {#if !selected.pending}
              <div class="commit-actions">
                <button type="button" class="danger" onclick={() => ask(`Reset hard a ${selected.id.slice(0, 7)}. El trabajo sin commit se pierde. Puedes volver al commit anterior en esta sesión.`, async () => { await call("reset_hard", { target: selected.id, confirm: true }); await loadRepo(); })}>Reset hard</button>
                <button type="button" onclick={() => fail(async () => { await call("undo_reset"); await loadRepo(); })}>Volver</button>
              </div>
              {/if}
            </article>
          </div>
        {/if}
        {/if}
      </aside>
    </div>
  {/if}
    </div>
  </div>

  {#if logText !== null}
    <div class="modal-back" role="presentation" onclick={() => (logText = null)}>
      <div class="modal log-modal" role="dialog" aria-label="Historial git log" onclick={(event) => event.stopPropagation()}>
        <p class="kicker">Historial</p>
        <h2>git log</h2>
        <pre class="log-text">{logText || "(sin commits)"}</pre>
        <div class="actions">
          <button onclick={() => (logText = null)}>Cerrar</button>
          <button onclick={openLog} disabled={logLoading}>Actualizar</button>
        </div>
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
          <button class="primary" onclick={() => { const menu = dragMenu; dragMenu = null; fail(async () => { await call("switch_branch", { name: menu.branch }); info = { ...info, branch: menu.branch }; await runMerge(menu.source); }); }}>Merge</button>
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

  {#if showBusy}
    <div class="busy-back" role="status" aria-live="polite" aria-busy="true">
      {@render mangoWalk()}
      <p class="kicker">Trabajando</p>
    </div>
  {/if}
</div>

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
      {#each branchMenuItems(branchMenu.current, branchMenu.branch, info?.branch || "").filter((item) => !(branchMenu.remote && item.id === "switch")) as item (item.id)}
        <button type="button" role="menuitem" onclick={() => runBranchAction(item.id)}>
          {@render icon(item.icon)}
          <span class="menu-copy">
            <span>{item.label}</span>
            {#if item.flow}<small class="flow">{item.flow}</small>{/if}
          </span>
        </button>
      {/each}
    </div>
  </div>
{/if}

{#if commitMenu}
  <div
    class="context-back"
    role="presentation"
    onclick={() => (commitMenu = null)}
    oncontextmenu={(event) => { event.preventDefault(); commitMenu = null; }}
    onkeydown={(event) => { if (event.key === "Escape") commitMenu = null; }}
  >
    <div
      class="context"
      role="menu"
      tabindex="-1"
      style:left={`${commitMenu.x}px`}
      style:top={`${commitMenu.y}px`}
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => { if (event.key === "Escape") commitMenu = null; }}
    >
      <p class="kicker">Commit</p>
      <p class="context-name">{commitMenu.commit.id.slice(0, 7)} · {commitMenu.commit.summary}</p>
      {#each commitMenuItems(commitMenu.commit) as item (item.id)}
        <button type="button" role="menuitem" class:danger={item.danger} onclick={() => runCommitAction(item.id)}>
          {@render icon(item.icon)}
          <span>{item.label}</span>
        </button>
      {/each}
    </div>
  </div>
{/if}

<svelte:body></svelte:body>
