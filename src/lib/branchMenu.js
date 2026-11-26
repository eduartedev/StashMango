export function branchFlow(from, to) {
  const left = String(from || "").trim() || "esta rama";
  const right = String(to || "").trim() || "rama actual";
  return `${left} → ${right}`;
}

export function branchMenuItems(isCurrent, target = "", onto = "") {
  const from = String(target || "").trim() || "esta rama";
  const here = String(onto || "").trim() || "rama actual";
  const copy = { id: "copy", label: "Copiar nombre", icon: "copy" };
  const createTag = { id: "tag", label: "Crear etiqueta aquí", icon: "folder" };
  if (isCurrent) {
    return [
      copy,
      createTag,
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
    createTag,
    { id: "delete", label: "Eliminar", icon: "close", danger: true },
  ];
}

export function menuPosition(x, y, width, height, bounds) {
  const margin = 12;
  let finalX = Math.max(margin, Math.min(x, bounds.width - width - margin));
  let finalY = Math.max(margin, Math.min(y, bounds.height - height - margin));
  return { x: finalX, y: finalY };
}
