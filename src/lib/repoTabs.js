import { sameWorktreePath, worktreeFolder, worktreePathKey } from "./worktreeCard.js";

function familyPaths(path, worktrees) {
  const list = Array.isArray(worktrees) ? worktrees : [];
  if (!list.some((tree) => sameWorktreePath(tree?.path, path))) return [];
  return list.map((tree) => tree.path);
}

export function placeRepoTab(tabs, info, worktrees = [], autoGroup = true) {
  const path = info?.path || "";
  if (!path) return tabs || [];
  const family = autoGroup ? familyPaths(path, worktrees) : [];
  const id = worktreePathKey(family[0] || path);
  const nextTab = {
    id,
    path,
    name: worktreeFolder(family[0] || path),
    branch: info.branch || "",
  };
  const current = tabs || [];
  const index = current.findIndex(
    (item) => item.id === id || family.some((itemPath) => sameWorktreePath(itemPath, item.path)) || sameWorktreePath(item.path, path),
  );
  const placed = index >= 0 ? current.map((item, i) => (i === index ? nextTab : item)) : [...current, nextTab];
  if (!family.length) return placed;
  let kept = false;
  return placed.filter((item) => {
    const inside = item.id === id || family.some((itemPath) => sameWorktreePath(itemPath, item.path));
    if (!inside) return true;
    if (kept) return false;
    kept = true;
    return true;
  });
}
