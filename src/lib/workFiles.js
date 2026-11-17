export function fileName(path) {
  const parts = String(path || "").split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] || "";
}

export function fileFolder(path) {
  const parts = String(path || "").split(/[\\/]/).filter(Boolean);
  return parts.length > 1 ? parts.slice(0, -1).join("/") : "";
}

export function groupWorkFiles(files) {
  const groups = {
    conflict: [],
    staged: [],
    unstaged: [],
    untracked: [],
  };
  for (const file of files || []) {
    if (file.conflict) groups.conflict.push(file);
    else if (file.untracked) groups.untracked.push(file);
    else {
      if (file.staged) groups.staged.push(file);
      if (file.unstaged) groups.unstaged.push(file);
    }
  }
  return [
    { id: "conflict", title: "Conflictos", files: groups.conflict },
    { id: "staged", title: "En el índice", files: groups.staged },
    { id: "unstaged", title: "Cambios", files: groups.unstaged },
    { id: "untracked", title: "Sin seguimiento", files: groups.untracked },
  ].filter((group) => group.files.length);
}
