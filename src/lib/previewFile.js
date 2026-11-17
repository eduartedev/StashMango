export function previewKicker(diff) {
  if (!diff) return "Archivo";
  if (diff.sha) return "Commit";
  if (diff.kind === "image") return "Imagen";
  if (diff.kind === "text") return "Archivo";
  if (diff.cached) return "En el índice";
  return "Cambios";
}

export function canStagePreview(diff) {
  return Boolean(diff && !diff.sha && (diff.kind || "diff") === "diff" && !diff.binary && !diff.too_big);
}

export function previewImageSrc(diff) {
  const media = String(diff?.media || "");
  if (!diff?.data || !/^image\/(png|jpeg|gif|webp|bmp)$/.test(media)) return "";
  return `data:${media};base64,${diff.data}`;
}
