export const defaultTemplates = {
  commit: "",
  merge: "Merge branch '{branch}' into {current}",
  stash: "WIP on {current}: {summary}",
  cherry: "Cherry-pick {sha} onto {current}",
  rebase: "Rebase {current} onto {branch}",
  reword: "{summary}",
  pr: "[{ticket}] {summary}",
};

export const templateKinds = [
  { id: "commit", label: "Commit", hint: "Texto inicial del recuadro de trabajo", icon: "template" },
  { id: "merge", label: "Merge", hint: "Commit al fusionar otra rama", icon: "merge" },
  { id: "stash", label: "Stash", hint: "Al guardar el trabajo a medias", icon: "stash" },
  { id: "cherry", label: "Cherry-pick", hint: "Al traer un commit a otra rama", icon: "pr" },
  { id: "rebase", label: "Rebase", hint: "Texto de confirmación", icon: "rebase" },
  { id: "reword", label: "Reword", hint: "Al reescribir en el rebase visual", icon: "switch" },
  { id: "pr", label: "Pull request", hint: "Título si el campo está vacío", icon: "pr" },
];

export const templateTokens = [
  { id: "branch", label: "Rama que entra" },
  { id: "current", label: "Rama actual" },
  { id: "ticket", label: "Ticket" },
  { id: "type", label: "Tipo" },
  { id: "scope", label: "Ámbito" },
  { id: "files", label: "Archivos" },
  { id: "count", label: "Cuántos" },
  { id: "sha", label: "Hash" },
  { id: "summary", label: "Resumen" },
  { id: "date", label: "Fecha" },
];

export function normalizeTemplates(saved = {}) {
  const next = {};
  for (const key of Object.keys(defaultTemplates)) {
    next[key] = saved[key] == null ? defaultTemplates[key] : String(saved[key]);
  }
  return next;
}

export function ticketFrom(name) {
  const match = String(name || "").match(/[A-Z][A-Z0-9]+-\d+/i);
  return match ? match[0].toUpperCase() : "";
}

export function typeFrom(name) {
  const head = String(name || "")
    .split(/[/_-]/)
    .find(Boolean)
    ?.toLowerCase() || "";
  const aliases = { feature: "feat", bugfix: "fix", hotfix: "fix", bug: "fix" };
  if (aliases[head]) return aliases[head];
  const known = ["feat", "fix", "chore", "docs", "test", "refactor", "style", "ci"];
  return known.includes(head) ? head : "";
}

export function scopeFrom(paths) {
  const folders = (paths || [])
    .map((path) => String(path).split(/[\\/]/).filter(Boolean)[0] || "")
    .filter(Boolean);
  if (!folders.length) return "";
  return folders.every((folder) => folder === folders[0]) ? folders[0] : "";
}

export function filesList(paths, limit = 3) {
  const names = (paths || []).map((path) => String(path).split(/[\\/]/).pop()).filter(Boolean);
  if (!names.length) return "";
  if (names.length <= limit) return names.join(", ");
  return `${names.slice(0, limit).join(", ")} +${names.length - limit}`;
}

export function templateVars(input = {}) {
  const paths = (input.files || [])
    .map((item) => (typeof item === "string" ? item : item?.path || ""))
    .filter(Boolean);
  const now = input.now instanceof Date ? input.now : new Date();
  const branch = String(input.branch || "");
  const current = String(input.current || "");
  return {
    branch,
    current,
    ticket: ticketFrom(branch) || ticketFrom(current),
    type: typeFrom(branch) || typeFrom(current) || typeFrom(input.summary),
    scope: scopeFrom(paths),
    files: filesList(paths),
    count: paths.length ? String(paths.length) : "",
    sha: String(input.sha || "").slice(0, 7),
    summary: String(input.summary || ""),
    date: now.toLocaleDateString("es", { day: "2-digit", month: "short", year: "numeric" }),
  };
}

export function fillTemplate(template, vars = {}) {
  let text = String(template || "").replace(/\{([a-z]+)\}/gi, (all, key) => {
    const value = vars[String(key).toLowerCase()];
    if (value == null || value === "") return "";
    return String(value);
  });
  return text
    .replace(/\[\s*\]/g, "")
    .replace(/\(\s*\)/g, "")
    .replace(/\s{2,}/g, " ")
    .replace(/^[:\-–,\s]+|[:\-–,\s]+$/g, "")
    .trim();
}

export function sampleVars(info, files) {
  const paths = (files || []).map((item) => item.path || item).filter(Boolean);
  return templateVars({
    branch: "feature/JIRA-12-login",
    current: info?.branch || "develop",
    files: paths.length ? paths : ["Construccion/estilo.css", "Construccion/index.html", "Construccion/AGENTS.md"],
    sha: "a1b2c3d",
    summary: "arregla el login",
    now: new Date(2026, 10, 17),
  });
}

export function wrapToken(id) {
  return `{${id}}`;
}

export function insertToken(text, token, start, end) {
  const current = String(text || "");
  const from = Math.max(0, Math.min(current.length, Number(start) || 0));
  const to = Math.max(from, Math.min(current.length, Number(end) || from));
  const piece = wrapToken(token);
  return { text: `${current.slice(0, from)}${piece}${current.slice(to)}`, cursor: from + piece.length };
}
