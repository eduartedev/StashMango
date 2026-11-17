const limits = {
  sideWidth: [180, 480],
  detailWidth: [240, 560],
  refsWidth: [140, 420],
  graphWidth: [64, 720],
};

export const graphColumnChoices = [
  { id: "refs", label: "Ramas", hint: "Nombre e iconos, a la izquierda del grafo" },
  { id: "graph", label: "Grafo", hint: "Líneas y commits" },
  { id: "message", label: "Mensaje", hint: "El texto del commit" },
  { id: "author", label: "Autor", hint: "Quién lo escribió" },
  { id: "date", label: "Fecha", hint: "Hace cuánto" },
  { id: "hash", label: "Hash", hint: "Los siete primeros caracteres" },
];

const graphColumnOrder = graphColumnChoices.map((item) => item.id);
const defaultGraphColumns = ["refs", "graph", "message", "date"];

export function graphColumnsOr(saved) {
  const known = new Set(graphColumnOrder);
  const picked = Array.isArray(saved) ? saved.filter((id) => known.has(id)) : null;
  const list = picked && picked.length ? picked : defaultGraphColumns;
  return graphColumnOrder.filter((id) => list.includes(id));
}

export function toggleGraphColumn(current, id) {
  if (!graphColumnOrder.includes(id)) return graphColumnsOr(current);
  const has = current.includes(id);
  if (has && current.length === 1) return graphColumnsOr(current);
  const next = has ? current.filter((item) => item !== id) : [...current, id];
  return graphColumnsOr(next);
}

export function columnTemplate(fields, refsWidth, graphWidth) {
  const size = {
    refs: `${refsWidth}px`,
    graph: `${graphWidth}px`,
    message: "minmax(220px, 1fr)",
    author: "150px",
    date: "120px",
    hash: "88px",
  };
  return graphColumnsOr(fields).map((id) => size[id]).join(" ");
}

export function clampColumn(name, value) {
  const [min, max] = limits[name];
  const number = Number(value);
  if (!Number.isFinite(number)) return min;
  return Math.round(Math.min(max, Math.max(min, number)));
}

export function normalizeColumns(saved = {}) {
  return {
    side: saved.side !== false,
    detail: saved.detail !== false,
    sideWidth: clampColumn("sideWidth", saved.sideWidth ?? saved.side_width ?? 268),
    detailWidth: clampColumn("detailWidth", saved.detailWidth ?? saved.detail_width ?? 360),
    refsWidth: clampColumn("refsWidth", saved.refsWidth ?? saved.refs_width ?? 220),
    graphWidth: clampColumn("graphWidth", saved.graphWidth ?? saved.graph_width ?? 160),
    graphColumns: graphColumnsOr(saved.graphColumns ?? saved.graph_columns),
  };
}

export function layoutPayload(columns) {
  return {
    side: columns.side,
    detail: columns.detail,
    side_width: columns.sideWidth,
    detail_width: columns.detailWidth,
    refs_width: columns.refsWidth,
    graph_width: columns.graphWidth,
    graph_columns: graphColumnsOr(columns.graphColumns),
  };
}
