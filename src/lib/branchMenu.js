export function branchMenuItems(isCurrent) {
  const copy = { id: "copy", label: "Copiar nombre", icon: "copy" };
  if (isCurrent) {
    return [
      copy,
      { id: "fetch", label: "Fetch", icon: "fetch" },
      { id: "pull", label: "Pull", icon: "pull" },
      { id: "push", label: "Push", icon: "push" },
    ];
  }
  return [
    { id: "switch", label: "Cambiar a esta rama", icon: "switch" },
    { id: "merge", label: "Merge en la rama actual", icon: "merge" },
    { id: "rebase", label: "Rebase de la actual sobre esta", icon: "rebase" },
    copy,
  ];
}

export function menuPosition(x, y, width, height, bounds) {
  return {
    x: Math.max(8, Math.min(x, bounds.width - width - 8)),
    y: Math.max(8, Math.min(y, bounds.height - height - 8)),
  };
}
