export const sideNav = [
  { id: "ramas", label: "Ramas", icon: "branch" },
  { id: "worktrees", label: "Worktrees", icon: "worktree" },
  { id: "stash", label: "Stash", icon: "stash" },
  { id: "prs", label: "Pull requests", icon: "pr" },
];

export function nextSide(open, current, clicked) {
  const known = sideNav.some((item) => item.id === clicked);
  const view = known ? clicked : "ramas";
  if (open && current === view) return { open: false, view };
  return { open: true, view };
}
