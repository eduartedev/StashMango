export function captionToggleTarget(target) {
  if (!target || typeof target !== "object") return false;
  const name = target.classList;
  if (!name || typeof name.contains !== "function") return false;
  return name.contains("top") || name.contains("spacer");
}
