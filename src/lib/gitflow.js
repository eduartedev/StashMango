export const gitflowKinds = [
  { id: "production", label: "Production", hint: "Ramas estables", def: "main, master" },
  { id: "develop", label: "Develop", hint: "Integración", def: "develop" },
  { id: "feature", label: "Feature", hint: "Prefijo o nombres", def: "feature/" },
  { id: "release", label: "Release", hint: "Prefijo o nombres", def: "release/" },
  { id: "hotfix", label: "Hotfix", hint: "Prefijo o nombres", def: "hotfix/" },
  { id: "support", label: "Support", hint: "Prefijo o nombres", def: "support/" },
];

export function defaultGitflow() {
  const next = { enabled: false };
  for (const kind of gitflowKinds) {
    next[kind.id] = kind.def;
  }
  return next;
}

export function parseGitflowTokens(text) {
  return String(text || "")
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

export function gitflowTokenOk(token) {
  if (!token || token.length > 120) return false;
  if (token.startsWith("-") || token.startsWith(".") || token.startsWith("/")) return false;
  if (token.includes("..") || token.includes("//")) return false;
  const body = token.endsWith("/") ? token.slice(0, -1) : token;
  if (!body) return false;
  return /^[A-Za-z0-9/_.-]+$/.test(body);
}

export function assertGitflow(flow) {
  const next = normalizeGitflow(flow);
  for (const kind of gitflowKinds) {
    for (const token of parseGitflowTokens(flow?.[kind.id])) {
      if (!gitflowTokenOk(token)) {
        throw new Error("Hay un nombre de rama que no es válido.");
      }
    }
  }
  return next;
}

export function normalizeGitflow(saved = {}) {
  if (saved === true) saved = { enabled: true };
  if (!saved || saved === false) saved = {};
  const next = defaultGitflow();
  next.enabled = saved.enabled === true || saved.enabled === "true";
  for (const kind of gitflowKinds) {
    const tokens = parseGitflowTokens(saved[kind.id]);
    next[kind.id] = tokens.length ? tokens.join(", ") : kind.def;
  }
  return next;
}

function tokenMatches(name, token) {
  if (token.endsWith("/")) {
    const prefix = token.slice(0, -1);
    if (!prefix) return false;
    return name === prefix || name.startsWith(token);
  }
  return name === token;
}

export function gitflowBucket(short, rules = defaultGitflow()) {
  const name = String(short || "");
  const config = normalizeGitflow({ ...rules, enabled: true });
  for (const kind of gitflowKinds) {
    for (const token of parseGitflowTokens(config[kind.id])) {
      if (!tokenMatches(name, token)) continue;
      if (token.endsWith("/") && name.startsWith(token)) {
        return { id: kind.id, rest: name.slice(token.length) || token.slice(0, -1) };
      }
      return { id: kind.id, rest: name };
    }
  }
  return { id: "otras", rest: name };
}
