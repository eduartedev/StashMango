import assert from "node:assert/strict";
import test from "node:test";
import { clampColumn, columnTemplate, graphColumnsOr, layoutPayload, normalizeColumns, toggleGraphColumn } from "./columns.js";

test("un ancho fuera de rango se recorta", () => {
  assert.equal(clampColumn("sideWidth", 20), 180);
  assert.equal(clampColumn("refsWidth", 900), 420);
  assert.equal(clampColumn("graphWidth", 2000), 720);
  assert.equal(clampColumn("detailWidth", "no"), 240);
});

test("un diseño viejo, sin anchos, abre las columnas con la medida de inicio", () => {
  const columns = normalizeColumns({ side: false, detail: true });
  assert.equal(columns.side, false);
  assert.equal(columns.sideWidth, 268);
  assert.equal(columns.refsWidth, 220);
  assert.equal(columns.graphWidth, 160);
});

test("el guardado usa los nombres que espera el programa", () => {
  const payload = layoutPayload(normalizeColumns({ side_width: 300, detail_width: 400, refs_width: 180 }));
  assert.deepEqual(payload, {
    side: true,
    detail: true,
    side_width: 300,
    detail_width: 400,
    refs_width: 180,
    graph_width: 160,
    graph_columns: ["refs", "graph", "message", "date"],
  });
});

test("las columnas del grafo se agregan, se quitan y no se quedan vacías", () => {
  assert.deepEqual(graphColumnsOr(undefined), ["refs", "graph", "message", "date"]);
  assert.deepEqual(graphColumnsOr(["hash", "no", "refs"]), ["refs", "hash"]);
  assert.deepEqual(toggleGraphColumn(["refs", "message"], "author"), ["refs", "message", "author"]);
  assert.deepEqual(toggleGraphColumn(["refs", "graph", "message", "date"], "date"), ["refs", "graph", "message"]);
  assert.deepEqual(toggleGraphColumn(["message"], "message"), ["message"]);
  assert.equal(columnTemplate(["message", "refs"], 200, 160), "200px minmax(220px, 1fr)");
});
