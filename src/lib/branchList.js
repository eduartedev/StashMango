import { gitflowBucket, gitflowKinds, normalizeGitflow } from "./gitflow.js";

const GITFLOW_ORDER = [...gitflowKinds.map((kind) => kind.id), "otras"];
const KIND_LABEL = {
  ...Object.fromEntries(gitflowKinds.map((kind) => [kind.id, kind.label])),
  otras: "Otras",
};

function asRef(ref) {
  if (typeof ref === "string") {
    const name = ref.trim();
    return { name, sha: "", remote: name.startsWith("origin/") };
  }
  const name = String(ref?.name || "").trim();
  return { name, sha: String(ref?.sha || "").trim(), remote: Boolean(ref?.remote) };
}

function collapseOrigin(refs) {
  const list = (Array.isArray(refs) ? refs : []).map(asRef).filter((ref) => ref.name);
  const localSha = new Map();
  for (const ref of list) {
    if (!ref.remote) localSha.set(ref.name, ref.sha);
  }
  const hide = new Set();
  const synced = new Set();
  for (const ref of list) {
    if (!ref.remote || !ref.name.startsWith("origin/")) continue;
    const local = ref.name.slice("origin/".length);
    if (!local || local === "HEAD" || local.endsWith("/HEAD")) continue;
    const sha = localSha.get(local);
    if (sha && ref.sha && sha === ref.sha) {
      hide.add(ref.name);
      synced.add(local);
    }
  }
  return list
    .filter((ref) => !(ref.remote && hide.has(ref.name)))
    .map((ref) => ({ ...ref, synced: synced.has(ref.name) }));
}

function splitRemote(name) {
  const slash = name.indexOf("/");
  if (slash <= 0) return null;
  const remote = name.slice(0, slash);
  const rest = name.slice(slash + 1);
  if (!rest || rest === "HEAD" || rest.endsWith("/HEAD")) return null;
  return { remote, rest };
}

function branchTree(items) {
  const root = { children: [], leaves: [] };
  const index = new Map();
  for (const item of [...items].sort((a, b) => a.name.localeCompare(b.name))) {
    const parts = item.name.split("/").filter(Boolean);
    if (parts.length <= 1) {
      root.leaves.push(item);
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
    parent.leaves.push(item);
  }
  return root;
}

function leafCount(node) {
  return node.leaves.length + node.children.reduce((sum, child) => sum + leafCount(child), 0);
}

function folderShut(closed, path, reveal) {
  if (reveal) return false;
  return closed[path] !== false;
}

function walkTree(node, depth, prefix, closed, rows, reveal) {
  for (const child of node.children) {
    const path = prefix ? `${prefix}/${child.name}` : child.path;
    const count = leafCount(child);
    if (!count) continue;
    const shut = folderShut(closed, path, reveal);
    rows.push({ kind: "folder", path, name: child.name, depth, shut, count });
    if (!shut) walkTree(child, depth + 1, path, closed, rows, reveal);
  }
  for (const leaf of node.leaves) {
    rows.push({
      kind: "leaf",
      branch: leaf.full,
      depth,
      label: leaf.name.includes("/") ? leaf.name.split("/").pop() : leaf.name,
      synced: Boolean(leaf.synced),
      remote: Boolean(leaf.remote),
    });
  }
}

function toItems(refs, shortKey) {
  return refs.map((ref) => ({
    name: shortKey(ref),
    full: ref.name,
    synced: Boolean(ref.synced),
    remote: Boolean(ref.remote),
  }));
}

function pushGroup(rows, path, name, refs, closed, reveal) {
  if (!refs.length) return;
  const shut = folderShut(closed, path, reveal);
  rows.push({ kind: "folder", path, name, depth: 0, shut, count: refs.length, remote: name !== "local" });
  if (shut) return;
  const items = toItems(refs, (ref) => (ref.short || ref.name));
  walkTree(branchTree(items), 1, path, closed, rows, reveal);
}

function splitSources(list) {
  const locals = [];
  const remotes = new Map();
  for (const ref of list) {
    if (!ref.remote) {
      locals.push(ref);
      continue;
    }
    const parts = splitRemote(ref.name);
    if (!parts) continue;
    if (!remotes.has(parts.remote)) {
      remotes.set(parts.remote, []);
    }
    remotes.get(parts.remote).push({ ...ref, short: parts.rest });
  }
  return { locals, remotes };
}

function gitflowRows(locals, remotes, closed, rules, reveal) {
  const buckets = new Map(GITFLOW_ORDER.map((id) => [id, { count: 0, sources: new Map() }]));
  const add = (refs, source) => {
    for (const ref of refs) {
      const { id, rest } = gitflowBucket(ref.short || ref.name, rules);
      const bucket = buckets.get(id);
      bucket.count += 1;
      if (!bucket.sources.has(source)) bucket.sources.set(source, []);
      bucket.sources.get(source).push({
        name: rest,
        full: ref.name,
        synced: Boolean(ref.synced),
        remote: Boolean(ref.remote),
      });
    }
  };
  add(locals, "local");
  const remoteNames = [...remotes.keys()].sort();
  for (const name of remoteNames) add(remotes.get(name), name);
  const rows = [];
  for (const id of GITFLOW_ORDER) {
    const bucket = buckets.get(id);
    if (!bucket.count) continue;
    const shut = folderShut(closed, id, reveal);
    rows.push({ kind: "folder", path: id, name: KIND_LABEL[id] || id, depth: 0, shut, count: bucket.count, kindId: id });
    if (shut) continue;
    const sources = [...bucket.sources.entries()].sort(([a], [b]) =>
      a === "local" ? -1 : b === "local" ? 1 : a.localeCompare(b),
    );
    for (const [source, items] of sources) {
      const path = `${id}/${source}`;
      const sourceShut = folderShut(closed, path, reveal);
      rows.push({
        kind: "folder",
        path,
        name: source,
        depth: 1,
        shut: sourceShut,
        count: items.length,
        remote: source !== "local",
      });
      if (!sourceShut) walkTree(branchTree(items), 2, path, closed, rows, reveal);
    }
  }
  return rows;
}

export function localBranchNames(refs) {
  return (Array.isArray(refs) ? refs : []).map(asRef).filter((ref) => ref.name && !ref.remote).map((ref) => ref.name);
}

export function matchesBranch(name, query) {
  const needle = String(query || "").trim().toLowerCase();
  if (!needle) return true;
  return String(name || "").toLowerCase().includes(needle);
}

export function branchRows(refs, closed = {}, options = {}) {
  const rules = normalizeGitflow(options.gitflow);
  const query = String(options.query || "").trim();
  const reveal = Boolean(query);
  let list = collapseOrigin(refs);
  if (query) {
    list = list.filter((ref) => matchesBranch(ref.name, query));
  }
  const { locals, remotes } = splitSources(list);
  const sections = [];
  sections.push({ id: "local", label: "LOCAL", rows: sectionRows(locals, "local", closed, rules, reveal) });
  for (const name of [...remotes.keys()].sort()) {
    sections.push({ id: name, label: name.toUpperCase(), rows: sectionRows(remotes.get(name), name, closed, rules, reveal) });
  }
  return sections;
}

function sectionRows(refs, name, closed, rules, reveal) {
  if (!refs.length) return [];
  const shut = folderShut(closed, name, reveal);
  const rows = [];
  if (rules.enabled) {
    const gitflowRows = gitflowSubRows(refs, name, closed, rules, reveal);
    rows.push(...gitflowRows);
  } else {
    const items = toItems(refs, (ref) => ref.short || ref.name);
    const tree = branchTree(items);
    walkTree(tree, 0, "", closed, rows, reveal);
  }
  return rows;
}

function gitflowSubRows(refs, parentPath, closed, rules, reveal) {
  const buckets = new Map(GITFLOW_ORDER.map((id) => [id, []]));
  for (const ref of refs) {
    const fullName = ref.short || ref.name;
    const { id, rest } = gitflowBucket(fullName, rules);
    buckets.get(id).push({
      name: rest,
      full: ref.name,
      synced: Boolean(ref.synced),
      remote: Boolean(ref.remote),
    });
  }
  const rows = [];
  for (const id of GITFLOW_ORDER) {
    const list = buckets.get(id);
    if (!list.length) continue;
    const path = `${parentPath}/${id}`;
    const shut = folderShut(closed, path, reveal);
    rows.push({ kind: "folder", path, name: KIND_LABEL[id] || id, depth: 0, shut, count: list.length, kindId: id });
    if (!shut) walkTree(branchTree(list), 1, path, closed, rows, reveal);
  }
  return rows;
}
