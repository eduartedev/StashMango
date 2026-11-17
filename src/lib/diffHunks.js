export function diffHunks(text) {
  const hunks = [];
  let current = null;
  for (const line of String(text || "").split("\n")) {
    if (line.startsWith("@@")) {
      current = { header: line, lines: [] };
      hunks.push(current);
      continue;
    }
    if (!current) continue;
    if (line.startsWith("+") || line.startsWith("-") || line.startsWith(" ") || line.startsWith("\\")) {
      current.lines.push(line);
    }
  }
  return hunks;
}

export function lineKind(line) {
  const prefix = String(line || "")[0];
  if (prefix === "+") return "add";
  if (prefix === "-") return "del";
  return "ctx";
}

export function stageableIndex(lines, lineIndex) {
  const line = lines[lineIndex];
  if (lineKind(line) !== "add" && lineKind(line) !== "del") return -1;
  let index = -1;
  for (let i = 0; i <= lineIndex; i += 1) {
    const kind = lineKind(lines[i]);
    if (kind === "add" || kind === "del") index += 1;
  }
  return index;
}
