export function branchFlow(from, to) {
  const left = String(from || "").trim() || "esta rama";
  const right = String(to || "").trim() || "rama actual";
  return `${left} → ${right}`;
}

export function branchMenuItems(isCurrent, target = "", onto = "") {
  const from = String(target || "").trim() || "esta rama";
  const here = String(onto || "").trim() || "rama actual";
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
    { id: "switch", label: "Cambiar", flow: branchFlow(here, from), icon: "switch" },
    { id: "merge", label: "Merge", flow: branchFlow(from, here), icon: "merge" },
    { id: "rebase", label: "Rebase", flow: branchFlow(here, from), icon: "rebase" },
    copy,
  ];
}

export function menuPosition(x, y, width, height, bounds) {
  return {
    x: Math.max(8, Math.min(x, bounds.width - width - 8)),
    y: Math.max(8, Math.min(y, bounds.height - height - 8)),
  };
}
