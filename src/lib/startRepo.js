const MAX_CLONE = 500;

export function cloneUrlOk(raw) {
  const text = String(raw || "");
  if (/[\n\r\0]/.test(text)) return false;
  const url = text.trim();
  if (!url || url.length > MAX_CLONE) return false;
  if (url.startsWith("-")) return false;
  if (url.toLowerCase().includes("ext::")) return false;
  if (/^https:\/\/[A-Za-z0-9.-]+(?::[0-9]+)?(?:\/[A-Za-z0-9._/~%+-]*)?$/i.test(url)) return true;
  if (/^ssh:\/\/[A-Za-z0-9._-]+@[A-Za-z0-9.-]+(?::[0-9]+)?\/[A-Za-z0-9._/~%+-]+$/i.test(url)) return true;
  if (/^[A-Za-z0-9._-]+@[A-Za-z0-9.-]+:[A-Za-z0-9._/~+-]+$/.test(url)) return true;
  if (url.includes("..")) return false;
  return /^[A-Za-z]:[\\/]/.test(url) || url.startsWith("/") || url.startsWith("\\\\");
}

export function cloneNameFromUrl(raw) {
  const url = String(raw || "").trim().replace(/[\\/]+$/, "");
  const noGit = url.replace(/\.git$/i, "");
  const parts = noGit.split(/[\\/:]+/).filter(Boolean);
  const name = parts[parts.length - 1] || "";
  if (!name || name === "." || name === ".." || name.includes("..")) return "";
  if (!/^[A-Za-z0-9._~+-]+$/.test(name)) return "";
  return name;
}

export function joinDest(folder, name) {
  const root = String(folder || "").replace(/[\\/]+$/, "");
  if (!root) return "";
  if (!name) return root;
  const sep = root.includes("\\") ? "\\" : "/";
  return `${root}${sep}${name}`;
}

export function assertClone(url, dest) {
  if (!cloneUrlOk(url)) throw new Error("La URL no es válida.");
  if (!String(dest || "").trim()) throw new Error("Elige la carpeta destino.");
  if (/[\n\r\0]/.test(dest)) throw new Error("La carpeta no es válida.");
  return { url: String(url).trim(), dest: String(dest).trim() };
}
