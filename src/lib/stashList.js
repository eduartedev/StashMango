export function stashRefOk(id) {
  return /^stash@\{\d{1,6}\}$/.test(String(id || ""));
}

export function parseStash(line) {
  const text = String(line || "");
  const match = text.match(/^(stash@\{\d{1,6}\}):\s*(.*)$/);
  if (!match) return null;
  const id = match[1];
  const rest = match[2];
  const marked = rest.match(/^(?:WIP on|On) ([^:]+):\s*(.*)$/);
  return {
    id,
    index: Number(id.slice(7, -1)),
    branch: marked ? marked[1].trim() : "",
    title: (marked ? marked[2] : rest).trim() || rest,
    raw: text,
  };
}

export function stashRows(lines) {
  return (Array.isArray(lines) ? lines : []).map(parseStash).filter(Boolean);
}
