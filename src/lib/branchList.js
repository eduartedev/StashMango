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

export function localBranchNames(refs) {
  return (Array.isArray(refs) ? refs : []).map(asRef).filter((ref) => ref.name && !ref.remote).map((ref) => ref.name);
}

export function branchRows(refs, closed = {}) {
  const rows = [];
  function walk(node, depth) {
    for (const child of node.children) {
      const count = leafCount(child);
      if (!count) continue;
      const shut = closed[child.path] === true;
      rows.push({ kind: "folder", path: child.path, name: child.name, depth, shut, count });
      if (!shut) walk(child, depth + 1);
    }
    for (const leaf of node.leaves) {
      rows.push({
        kind: "leaf",
        branch: leaf.name,
        depth,
        label: leaf.name.includes("/") ? leaf.name.split("/").pop() : leaf.name,
        synced: Boolean(leaf.synced),
        remote: Boolean(leaf.remote),
      });
    }
  }
  walk(branchTree(collapseOrigin(refs)), 0);
  return rows;
}
