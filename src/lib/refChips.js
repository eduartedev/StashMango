export function refChips(refs) {
  if (!refs) return [];
  const chips = refs
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean)
    .flatMap((raw) => {
      if (raw.startsWith("HEAD -> ")) {
        const branch = raw.slice("HEAD -> ".length).trim();
        const chips = [{ kind: "head", label: "HEAD", title: raw }];
        if (branch) chips.push({ kind: "local", label: branch, title: branch });
        return chips;
      }
      if (raw.startsWith("tag: ")) {
        const label = raw.slice(5).trim();
        return [{ kind: "tag", label, title: raw }];
      }
      if (raw.includes("/")) return [{ kind: "remote", label: raw, title: raw }];
      return [{ kind: "local", label: raw, title: raw }];
    });
  return joinOrigin(chips);
}

function joinOrigin(chips) {
  const locals = new Set(chips.filter((chip) => chip.kind === "local").map((chip) => chip.label));
  const synced = new Set();
  for (const chip of chips) {
    if (chip.kind !== "remote" || !chip.label.startsWith("origin/")) continue;
    const local = chip.label.slice("origin/".length);
    if (!local || local === "HEAD" || local.endsWith("/HEAD") || !locals.has(local)) continue;
    synced.add(local);
  }
  return chips
    .filter((chip) => !(chip.kind === "remote" && chip.label.startsWith("origin/") && synced.has(chip.label.slice("origin/".length))))
    .map((chip) => (chip.kind === "local" && synced.has(chip.label) ? { ...chip, synced: true } : chip));
}

const rank = { local: 0, tag: 1, head: 2, remote: 3 };

export function packRefs(chips) {
  if (!chips.length) return { shown: null, extra: [] };
  if (chips.length === 1) return { shown: chips[0], extra: [] };
  const ranked = [...chips].sort((a, b) => (rank[a.kind] ?? 9) - (rank[b.kind] ?? 9));
  return { shown: ranked[0], extra: ranked.slice(1) };
}

export function chipKind(kind) {
  if (kind === "local") return "Rama";
  if (kind === "remote") return "Remoto";
  if (kind === "tag") return "Etiqueta";
  return "HEAD";
}

export function isCurrentRef(chip, branch) {
  if (!chip) return false;
  const current = (branch || "").trim();
  if (!current) return chip.kind === "head";
  return chip.kind === "local" && chip.label === current;
}

export function refTarget(chip, branches) {
  if (!chip) return "";
  const name = chip.kind === "local" ? chip.label : chip.kind === "remote" ? chip.label.split("/").slice(1).join("/") : "";
  if (!name || name === "HEAD" || !branches.includes(name)) return "";
  return name;
}
