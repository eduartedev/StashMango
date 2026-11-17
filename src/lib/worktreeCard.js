export function worktreeFolder(path) {
  const parts = String(path || "").split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] || "";
}

export function worktreeLabel(tree) {
  if (!tree) return "";
  const branch = String(tree.branch || "").trim();
  if (branch) return branch;
  const head = String(tree.head || "").trim();
  return head ? head.slice(0, 7) : "";
}

export function worktreePathKey(path) {
  return String(path || "").replace(/\\/g, "/").replace(/\/+$/, "");
}

export function sameWorktreePath(left, right) {
  const a = worktreePathKey(left);
  const b = worktreePathKey(right);
  return Boolean(a) && a === b;
}
